-- Your SQL goes here
ALTER TABLE submission
ADD CONSTRAINT unique_team_puzzle UNIQUE (team, puzzle);