/**
 * POST /api/v1/admin/import/game/auto-batch
 *
 * Automatically imports every unimported game in the library by:
 *   1. Normalising the folder name (strips release groups, version strings, dot-separators)
 *   2. Searching all configured metadata providers
 *   3. Queuing a game-import task for the top result if its fuzzy score meets the threshold
 *
 * Body (all optional):
 *   dryRun     – if true, report what *would* happen without creating any tasks (default: false)
 *   minScore   – minimum fast-fuzzy score [0–1] to auto-accept (default: 0.75)
 *   gameType   – GameType enum value to apply to every imported game (default: "Game")
 *   limit      – cap on how many games to process in one call, 0 = unlimited (default: 0)
 *
 * Response:
 *   {
 *     processed : number,          // games examined
 *     queued    : QueuedEntry[],   // games with a task created (or would-be in dry-run)
 *     skipped   : SkippedEntry[],  // games with no confident match
 *     dryRun    : boolean
 *   }
 */

import { type } from "arktype";
import { GameType } from "~/prisma/client/enums";
import { readDropValidatedBody, throwingArktype } from "~/server/arktype";
import aclManager from "~/server/internal/acls";
import libraryManager from "~/server/internal/library";
import metadataHandler from "~/server/internal/metadata";
import { normalizeGameName } from "~/server/internal/utils/gameNameNormalize";
import { logger } from "~/server/internal/logging";

const AutoBatchBody = type({
  "dryRun?": "boolean",
  "minScore?": "number",
  "gameType?": type.valueOf(GameType),
  "limit?": "number",
}).configure(throwingArktype);

export default defineEventHandler(async (h3) => {
  const allowed = await aclManager.allowSystemACL(h3, [
    "import:game:read",
    "import:game:new",
  ]);
  if (!allowed) throw createError({ statusCode: 403 });

  const body = await readDropValidatedBody(h3, AutoBatchBody);
  const dryRun = body.dryRun ?? false;
  const minScore = body.minScore ?? 0.75;
  const gameType = body.gameType ?? GameType.Game;
  const limit = body.limit ?? 0;

  // ── Gather all unimported games ──────────────────────────────────────────
  const unimportedMap = await libraryManager.fetchUnimportedGames();

  interface QueuedEntry {
    libraryId: string;
    path: string;
    normalizedName: string;
    matchedName: string;
    source: string;
    score: number;
    taskId?: string;
  }
  interface SkippedEntry {
    libraryId: string;
    path: string;
    normalizedName: string;
    reason: string;
    topScore?: number;
    topMatch?: string;
  }

  const queued: QueuedEntry[] = [];
  const skipped: SkippedEntry[] = [];
  let processed = 0;

  outer: for (const [libraryId, paths] of Object.entries(unimportedMap)) {
    for (const gamePath of paths) {
      if (limit > 0 && processed >= limit) break outer;
      processed++;

      const normalizedName = normalizeGameName(gamePath);

      // ── Search metadata providers ──────────────────────────────────────
      let results: Awaited<ReturnType<typeof metadataHandler.search>>;
      try {
        results = await metadataHandler.search(normalizedName);
      } catch (err) {
        logger.warn(
          { libraryId, gamePath, err },
          "auto-batch: metadata search failed",
        );
        skipped.push({
          libraryId,
          path: gamePath,
          normalizedName,
          reason: "metadata search error",
        });
        continue;
      }

      if (!results || results.length === 0) {
        skipped.push({
          libraryId,
          path: gamePath,
          normalizedName,
          reason: "no metadata results",
        });
        continue;
      }

      // Results are already sorted by fuzzy score descending (see MetadataHandler.search)
      const top = results[0];

      if ((top.fuzzy ?? 0) < minScore) {
        skipped.push({
          libraryId,
          path: gamePath,
          normalizedName,
          reason: `low confidence (score ${(top.fuzzy ?? 0).toFixed(3)} < ${minScore})`,
          topScore: top.fuzzy,
          topMatch: top.name,
        });
        continue;
      }

      // ── Queue the import task ──────────────────────────────────────────
      if (!dryRun) {
        try {
          const taskId = await metadataHandler.createGame(
            { sourceId: top.sourceId, id: top.id, name: top.name },
            libraryId,
            gamePath,
            gameType,
          );
          if (!taskId) {
            // createGame returns undefined when a duplicate task already exists
            skipped.push({
              libraryId,
              path: gamePath,
              normalizedName,
              reason: "duplicate — already queued or imported",
            });
            continue;
          }
          queued.push({
            libraryId,
            path: gamePath,
            normalizedName,
            matchedName: top.name,
            source: top.sourceId,
            score: top.fuzzy ?? 0,
            taskId,
          });
        } catch (err) {
          logger.warn(
            { libraryId, gamePath, err },
            "auto-batch: createGame task failed",
          );
          skipped.push({
            libraryId,
            path: gamePath,
            normalizedName,
            reason: "task creation error",
          });
        }
      } else {
        queued.push({
          libraryId,
          path: gamePath,
          normalizedName,
          matchedName: top.name,
          source: top.sourceId,
          score: top.fuzzy ?? 0,
        });
      }
    }
  }

  return { processed, queued, skipped, dryRun };
});
