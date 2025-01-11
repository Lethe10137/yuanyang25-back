import hashlib
import random
import string
import psycopg2
from psycopg2 import sql
from psycopg2.extras import execute_values
import test_util

random.seed(4723)


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

def insert_random_puzzle():

    # Connect to the PostgreSQL database
    try:
        conn = psycopg2.connect(database_url)
        cursor = conn.cursor()

        # Prepare the INSERT query
        query = sql.SQL("""
            INSERT INTO puzzle (unlock ,bounty, title, answer, key, content)
            VALUES (%s, %s, %s, %s, %s, %s)
            RETURNING id
        """)
        
        query2 = sql.SQL("""
            INSERT INTO mid_answer (puzzle, query, response)
            VALUES (%s, %s, %s)
        """)

        result = []

        for i in range(NUM_PUZZLES):
            
            bounty = random.randint(100, 10000)  # Random integer for bounty
            unlock = bounty // 3  # Random integer for bounty
            title = generate_random_string(10)  # Random string of length 10
            answer = generate_random_string(10)  # Random string of length 10
            key = generate_random_string(16)  # Random string of length 16
            content = generate_random_string(100)  # Random string of length 100

            cursor.execute(query, (unlock, bounty, title, answer, key, content))
            
            inserted_id = cursor.fetchone()[0]
            
            mid1, mid2 = generate_random_string(10), generate_random_string(10)
            
            result.append((answer, key, mid1, mid2, inserted_id))
            cursor.execute(query2, (inserted_id, mid1, generate_random_string(20)))
            cursor.execute(query2, (inserted_id, mid2, generate_random_string(20)))
            

        # Commit the transaction
        conn.commit()
        print("%d rows successfully inserted into the 'puzzle' table.", NUM_PUZZLES)
        return result

    except Exception as e:
        print("Error:", e)
    finally:
        if cursor:
            cursor.close()
        if conn:
            conn.close()
            
    return []
            
            




def insert_unlock(puzzles):
    # Connect to the database
    conn = psycopg2.connect(database_url)
    try:
        with conn:
            with conn.cursor() as cursor:
                query = sql.SQL("""
                INSERT INTO "unlock" ("team", "puzzle") VALUES (%s, %s);
                """)
                for team in range(1, NUM_TEAMS+1):
                    for puzzle in puzzles:
                        if random.random() < 0.3:
                            print(team, puzzle)
                            cursor.execute(query, (team, puzzle))
                            
            conn.commit()
                
    finally:
        conn.close()
        


if __name__ == "__main__":
    import subprocess
    import time
    subprocess.run(["diesel","migration","redo" ,"--all"])
    
    users = test_util.prepare_users(NUM_TEAMS * 3)
    puzzles = insert_random_puzzle()
    
    print(puzzles)
    

    insert_unlock([t[-1] for t in puzzles])
    
    
    
    s = test_util.login(users[0]["id"], users[0]["pw"])
    
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
    
    res = s.post(
        test_util.url + "/submit_answer", json= {
            "puzzle_id" : puzzle_id,
            "answer" : hashlib.sha256((puzzles[0][1] + puzzles[0][0]).encode()).hexdigest()
        }
    )
    print(res.text, res)
    

    for i in range(NUM_PUZZLES):
        res = s.get(test_util.url + "/decipher_key?puzzle_id={}".format(i))
        print(i, res.text, res)