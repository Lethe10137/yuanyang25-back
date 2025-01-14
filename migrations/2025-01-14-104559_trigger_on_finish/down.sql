-- Drop trigger from "submission" table
DROP TRIGGER IF EXISTS "trigger_final_meta_submission" ON "submission";

-- Drop trigger function
DROP FUNCTION IF EXISTS insert_final_meta_submission();

-- drop idx
DROP INDEX IF EXISTS idx_team_final;

-- Drop "final_meta_submission" table
DROP TABLE IF EXISTS "final_meta_submission";

-- Remove "meta" column from "submission" table
ALTER TABLE "submission"
DROP COLUMN IF EXISTS "meta";

