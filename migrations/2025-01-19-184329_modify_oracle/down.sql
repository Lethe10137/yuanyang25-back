-- This file should undo anything in `up.sql`

DROP INDEX IF EXISTS "oracle_index_active";
DROP INDEX IF EXISTS "oracle_index_update";
DROP INDEX IF EXISTS  "oracle_index_id_active";

ALTER TABLE "oracle"
DROP COLUMN "active";
