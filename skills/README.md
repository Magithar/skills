# skills/

One directory per skill, grouped by category.

- `engineering/` and `productivity/` are published to the plugin manifest
- `in-progress/` and `deprecated/` are excluded from it

Every skill needs `SKILL.md` with `name:` and `description:` frontmatter. Run
`node scripts/sync-plugin.mjs` after adding one.
