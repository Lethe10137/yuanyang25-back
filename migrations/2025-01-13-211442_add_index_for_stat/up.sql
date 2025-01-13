-- Your SQL goes here
CREATE INDEX idx_puzzle_id ON puzzle(id);
CREATE INDEX idx_submission_puzzle ON submission(puzzle);
CREATE INDEX idx_unlock_decipher ON unlock(decipher);