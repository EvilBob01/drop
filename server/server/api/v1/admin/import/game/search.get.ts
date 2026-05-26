import aclManager from "~/server/internal/acls";
import metadataHandler from "~/server/internal/metadata";
import { normalizeGameName } from "~/server/internal/utils/gameNameNormalize";

export default defineEventHandler(async (h3) => {
  const allowed = await aclManager.allowSystemACL(h3, ["import:game:read"]);
  if (!allowed) throw createError({ statusCode: 403 });

  const query = getQuery(h3);
  const raw = query.q?.toString();
  if (!raw)
    throw createError({ statusCode: 400, statusMessage: "Invalid search" });

  // Normalise the raw folder name before hitting metadata providers so that
  // queries like "A.Plague.Tale.Requiem.v1076-GOG" become "A Plague Tale Requiem".
  // If the caller already passes a clean name (manual search), normalisation
  // is a no-op for names that contain no release-group markers.
  const search = normalizeGameName(raw);

  const results = await metadataHandler.search(search);

  if (results.length == 0)
    throw createError({
      statusCode: 404,
      statusMessage: "No metadata provider returned search results.",
    });

  // Return the array directly so existing UI code (metadataResults.value = results)
  // continues to work unchanged.
  return results;
});
