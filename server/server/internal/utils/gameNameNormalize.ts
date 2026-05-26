/**
 * Normalises raw filesystem game folder names before passing to metadata providers.
 *
 * Handles the common naming patterns used by repack groups and digital storefronts:
 *
 *   "Age of Empires IV [FitGirl Repack]"                    → "Age of Empires IV"
 *   "A.Plague.Tale.Requiem.v1076-GOG"                       → "A Plague Tale Requiem"
 *   "Above.Snakes-TENOKE"                                   → "Above Snakes"
 *   "RimWorld.v1.4.3613-GOG"                                → "RimWorld"
 *   "Airships.Conquer.the.Skies.v1.1.2.1-GOG"              → "Airships Conquer the Skies"
 *   "Age.of.Mythology.Extended.Edition.Tale.of.the.Dragon.v2.7-PLAZA"
 *                                                           → "Age of Mythology Extended Edition Tale of the Dragon"
 *   "A Total War Saga - ToB [FitGirl Repack]"               → "A Total War Saga - ToB"
 *   "Age of Empires III - Complete Collection"              → "Age of Empires III - Complete Collection"
 */

// Known release group identifiers (lower-case for comparison)
const RELEASE_GROUPS = new Set([
  "fitgirl",
  "fitgirl repack",
  "gog",
  "codex",
  "skidrow",
  "plaza",
  "dodi",
  "cpy",
  "empress",
  "fltdox",
  "darksiders",
  "rune",
  "reloaded",
  "razor1911",
  "prophet",
  "goldberg",
  "tenoke",
  "tinyiso",
  "masquerade",
  "kaos",
  "revolt",
  "chronos",
  "kali",
  "hoodlum",
  "shi700",
  "rarbg",
  "yolo",
  "tcdiso",
  "drm free",
  "drm-free",
]);

// Terms inside brackets that indicate a group/repack label (not an edition/subtitle)
const REPACK_KEYWORDS = [
  "repack",
  "rip",
  "rom pack",
  "rom",
  "crack",
  "release",
];

function bracketContentIsGroup(inner: string): boolean {
  const lower = inner.toLowerCase().trim();
  // Check against known group names
  for (const g of RELEASE_GROUPS) {
    if (lower === g || lower.includes(g)) return true;
  }
  // Check against generic repack keywords
  for (const kw of REPACK_KEYWORDS) {
    if (lower.includes(kw)) return true;
  }
  return false;
}

// Trailing release group after a dash: "GameName-GOG", "GameName-CODEX", etc.
// Matches at end of string only.
const TRAILING_GROUP_RE = new RegExp(
  `-\\s*(?:${[...RELEASE_GROUPS]
    .map((g) => g.replace(/[.*+?^${}()|[\]\\]/g, "\\$&").replace(/\s+/g, "\\s*"))
    .join("|")}|repack|rip)\\s*$`,
  "gi",
);

// Version strings:
//   v1076, v1.4.3613, v2.7, 1.0.0.0, 1.0453
//   Must start with 'v' OR have multiple dot-separated segments.
//   Avoids stripping standalone numbers like "1" or "6666".
const VERSION_RE = /\bv\d[\d.]*\b|\b\d+(?:\.\d+){1,6}\b/gi;

export function normalizeGameName(raw: string): string {
  let name = raw;

  // 1. Strip square-bracket content that identifies a release group / repack
  name = name.replace(/\[([^\]]*)\]/g, (match, inner) =>
    bracketContentIsGroup(inner) ? "" : match,
  );

  // 2. Strip paren content that identifies a release group / repack
  name = name.replace(/\(([^)]*)\)/g, (match, inner) =>
    bracketContentIsGroup(inner) ? "" : match,
  );

  // 3. Strip trailing -GROUPNAME  (e.g. "-GOG", "-TENOKE")
  name = name.replace(TRAILING_GROUP_RE, "");

  // 4. Strip version strings *before* expanding dots so "v1.4.3613" doesn't
  //    become three separate space-separated tokens ("v1", "4", "3613").
  name = name.replace(VERSION_RE, "");

  // 5. Strip any newly exposed trailing group that was glued to the version
  name = name.replace(TRAILING_GROUP_RE, "");

  // 6. Expand dots that are acting as word separators.
  //    Heuristic: if the string has no spaces at all, every dot is a separator.
  //    If it has both spaces and dots, only replace dot-between-word-characters
  //    (catches "RimWorld.v1.4" residue but leaves "e.g." or "U.S." alone).
  if (name.includes(".")) {
    if (!name.includes(" ")) {
      // Fully dot-separated: replace all dots
      name = name.replace(/\./g, " ");
    } else {
      // Mixed: replace dot between two word characters
      name = name.replace(/(\w)\.(\w)/g, "$1 $2");
    }
  }

  // 7. Replace underscores with spaces
  name = name.replace(/_/g, " ");

  // 8. Strip trailing isolated dash (can appear after group/version removal)
  name = name.replace(/\s*-\s*$/, "");

  // 9. Collapse multiple spaces and trim
  name = name.replace(/\s+/g, " ").trim();

  return name;
}

/**
 * Convenience wrapper that returns both the original folder name and the
 * normalised search query in one call.
 */
export function normalizeGamePath(folderName: string): {
  original: string;
  normalized: string;
} {
  return {
    original: folderName,
    normalized: normalizeGameName(folderName),
  };
}
