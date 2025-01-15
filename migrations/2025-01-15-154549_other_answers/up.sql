-- Your SQL goes here

CREATE TABLE "other_answer" (
    "id" SERIAL PRIMARY KEY,
    "puzzle" INTEGER NOT NULL,
    "sha256" CHAR(64) NOT NULL,
    "content" TEXT NOT NULL,
    CONSTRAINT "fk_puzzle_other_answer"
        FOREIGN KEY ("puzzle") REFERENCES "puzzle" ("id")
        ON DELETE CASCADE
);

CREATE INDEX "other_answer_index_0"
ON "other_answer" ("puzzle");
