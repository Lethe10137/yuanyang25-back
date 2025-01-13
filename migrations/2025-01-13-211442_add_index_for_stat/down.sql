-- This file should undo anything in `up.sql`

DROP INDEX IF EXISTS idx_puzzle_id;
DROP INDEX IF EXISTS idx_submission_puzzle;
DROP INDEX IF EXISTS idx_unlock_decipher;
