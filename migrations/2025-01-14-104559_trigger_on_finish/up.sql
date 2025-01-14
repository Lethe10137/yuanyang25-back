
-- Add "meta" column to "submission" table
ALTER TABLE "submission"
ADD COLUMN "meta" BOOLEAN NOT NULL DEFAULT FALSE;

-- Create "final_meta_submission" table
CREATE TABLE "final_meta_submission" (
    "id" SERIAL PRIMARY KEY,
    "submission_id" INTEGER NOT NULL UNIQUE,
    "team" INTEGER NOT NULL,
    "puzzle" INTEGER NOT NULL,
    "reward" BIGINT NOT NULL,
    "time" TIMESTAMPTZ NOT NULL,
    CONSTRAINT "fk_submission_final_meta"
        FOREIGN KEY ("submission_id") REFERENCES "submission" ("id")
        ON DELETE CASCADE
);


-- create idx
CREATE INDEX idx_team_final ON final_meta_submission(team);

-- Create trigger function to insert into "final_meta_submission"
CREATE OR REPLACE FUNCTION insert_final_meta_submission()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.depth = 0 AND NEW.meta = TRUE THEN
        INSERT INTO "final_meta_submission" ("submission_id", "team", "puzzle", "reward", "time")
        VALUES (NEW.id, NEW.team, NEW.puzzle, NEW.reward, NEW.time);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger on "submission" table
CREATE TRIGGER "trigger_final_meta_submission"
AFTER INSERT ON "submission"
FOR EACH ROW
EXECUTE FUNCTION insert_final_meta_submission();
