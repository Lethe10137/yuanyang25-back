-- Your SQL goes here

CREATE TABLE "mid_answer_submission" (
	"id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
    "team" INTEGER NOT NULL,
	"mid_answer" INTEGER NOT NULL,
    "time" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY("id")
);

CREATE INDEX "mid_answer_submission_index"
ON "mid_answer_submission" ("team", "mid_answer");

ALTER TABLE "mid_answer_submission"
ADD CONSTRAINT unique_team_midanswer UNIQUE ("team", "mid_answer");

ALTER TABLE "mid_answer_submission"
ADD FOREIGN KEY("mid_answer") REFERENCES "mid_answer"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "mid_answer_submission"
ADD FOREIGN KEY("team") REFERENCES "team"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;


CREATE TABLE "wrong_answer_cnt" (
    "id" INTEGER NOT NULL UNIQUE GENERATED BY DEFAULT AS IDENTITY,
	"team" INTEGER NOT NULL,
	"puzzle" INTEGER NOT NULL,
    "token_penalty_level" INTEGER NOT NULL,
    "time_penalty_level" INTEGER NOT NULL,
    "time_penalty_until" TIMESTAMPTZ NOT NULL,
	PRIMARY KEY("id")
);

CREATE INDEX "wrong_answer_cnt_index"
ON "wrong_answer_cnt" ("team", "puzzle");

ALTER TABLE "wrong_answer_cnt"
ADD CONSTRAINT unique_team_wa UNIQUE ("team", "puzzle");

ALTER TABLE "wrong_answer_cnt"
ADD FOREIGN KEY("puzzle") REFERENCES "puzzle"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;
ALTER TABLE "wrong_answer_cnt"
ADD FOREIGN KEY("team") REFERENCES "team"("id")
ON UPDATE NO ACTION ON DELETE NO ACTION;