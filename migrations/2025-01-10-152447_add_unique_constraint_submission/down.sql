-- This file should undo anything in `up.sql`
ALTER TABLE submission
DROP CONSTRAINT unique_team_puzzle;
