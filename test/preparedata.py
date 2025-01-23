import hashlib
import random
import string
from typing import List
import psycopg2
from psycopg2 import sql
from psycopg2.extras import execute_values
import test_util
import json

random.seed(4724)


database_url = None
with open(".env", "r") as f:
    for line in f:
        try:
            name, value = line.strip().split("=")
            if(name == "DATABASE_URL"):
                database_url = value
        except:
            pass

if not database_url:
    raise ValueError("DATABASE_URL is not set in the .env file")
else:
    print(database_url)
    
import os

# Constants
NUM_PUZZLES = 10
NUM_TEAMS = 5


def generate_random_string(length=64):
    """Generate a random string of a given length."""
    return ''.join(random.choices(string.ascii_letters + string.digits, k=length))

def insert_mock_puzzle():
    
    backend_data = json.load(open("test/example_data/backend.json"))
    conn, cursor = None, None
    
    # Connect to the PostgreSQL database
    try:
        conn = psycopg2.connect(database_url)
        cursor = conn.cursor()

        # Prepare the INSERT query
        query1 = sql.SQL("""
            INSERT INTO puzzle (id, meta, bounty, title, decipher, depth)
            VALUES (%s, %s, %s, %s, %s, %s)
            RETURNING id
        """)
        
        query2 = sql.SQL("""
            INSERT INTO answer (puzzle, level, sha256)
            VALUES (%s, %s, %s)
        """)
        
        query3 = sql.SQL("""
            INSERT INTO other_answer (puzzle, sha256, content, ref)
            VALUES (%s, %s, %s, %s)
        """)

        result = []

        for puzzle in backend_data:
            
            puzzle_id = puzzle["puzzle_id"]
            title = puzzle["title"]
            meta = puzzle["meta"]
            bounty = puzzle["bounty"]
            decipher = puzzle["decipher_id"]
            
            answers: List[str] = puzzle["expected_cipher_answer"]
            other_answers : List[List[str]] = puzzle["other_cipher_answer_response"]

            cursor.execute(query1, (puzzle_id, meta, bounty, title, decipher, len(answers)))
            result.append(int(puzzle_id))
            
            for (level, answer) in enumerate(reversed(answers)):            
                cursor.execute(query2, (puzzle_id, level, answer))
                
            for (ref, (sha, response)) in enumerate(other_answers):
                cursor.execute(query3, (puzzle_id, sha, response, ref))
                
        # Commit the transaction
        conn.commit()

        return result

    except Exception as e:
        print("Error:", e)
    finally:
        if cursor:
            cursor.close()
        if conn:
            conn.close()
            
    return []
            
            




def insert_decipher():
    
    data = json.load(open("test/example_data/cipher_key.json"))
    
    # Connect to the database
    conn = psycopg2.connect(database_url)
    try:
        with conn:
            with conn.cursor() as cursor:
                query = sql.SQL("""
                INSERT INTO "decipher" ("id", "pricing_type", "base_price", "depth", "root") VALUES (%s, %s, %s, %s, %s);
                """)
                for item in data:
                    decipher_id, (sha, price, pricing_type, depth ) = item
                    cursor.execute(query, (decipher_id, pricing_type, price, depth, sha))
                            
            conn.commit()
                
    finally:
        conn.close()
        


if __name__ == "__main__":
    import subprocess
    import time
    subprocess.run(["diesel","migration","redo" ,"--all"])
    
    insert_mock_puzzle()
    insert_decipher()
    
    exit(0)
    
    users = test_util.prepare_users(NUM_TEAMS * 3)
    s = test_util.login(users[0]["id"], users[0]["pw"])
    
    
    
    exit(0)
    puzzle_id = puzzles[0][-1]
    
    res = s.post(
        test_util.url + "/submit_answer", json= {
            "puzzle_id" : puzzle_id,
            "answer" : hashlib.sha256((puzzles[0][1] + puzzles[0][2]).encode()).hexdigest()
        }
    )
    print(res.text, res)
    
    res = s.post(
        test_util.url + "/submit_answer", json= {
            "puzzle_id" : puzzle_id,
            "answer" : hashlib.sha256((puzzles[0][1] + puzzles[0][2]).encode()).hexdigest()
        }
    )
    print(res.text, res)
    
    res = s.post(
        test_util.url + "/submit_answer", json= {
            "puzzle_id" : puzzle_id,
            "answer" : hashlib.sha256((puzzles[0][1] + puzzles[0][3]).encode()).hexdigest()
        }
    )
    print(res.text, res)
  

    for i in range(10):
        res = s.post(
            test_util.url + "/submit_answer", json= {
                "puzzle_id" : puzzle_id,
                "answer" : hashlib.sha256((puzzles[0][0]).encode()).hexdigest()
            }
        )
        print(res.text, res)
        time.sleep(1)
    
    #首次提交正确答案
    res = s.post(
        test_util.url + "/submit_answer", json= {
            "puzzle_id" : puzzle_id,
            "answer" : hashlib.sha256((puzzles[0][1] + puzzles[0][0]).encode()).hexdigest()
        }
    )
    print(res.text, res)
    
    #重复提交正确答案
    res = s.post(
        test_util.url + "/submit_answer", json= {
            "puzzle_id" : puzzle_id,
            "answer" : hashlib.sha256((puzzles[0][1] + puzzles[0][0]).encode()).hexdigest()
        }
    )
    print(res.text, res)
    
    print("decipher")
    for i in range(NUM_PUZZLES):
        res = s.get(test_util.url + "/decipher_key?decipher_id={}".format(i+1))
        print(i+1, res.text, res)
        
        
    print("unlock")
    for i in range(NUM_PUZZLES):
        res = s.post(test_util.url + "/unlock?decipher_id={}".format(i+1))
        print(i+1, res.text, res)