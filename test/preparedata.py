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
        """)


        for _ in range(NUM_PUZZLES):
            
            bounty = random.randint(100, 10000)  # Random integer for bounty
            unlock = bounty // 3  # Random integer for bounty
            title = generate_random_string(10)  # Random string of length 10
            answer = generate_random_string(10)  # Random string of length 10
            key = generate_random_string(16)  # Random string of length 16
            content = generate_random_string(100)  # Random string of length 100

            cursor.execute(query, (bounty, title, answer, key, content))

        # Commit the transaction
        conn.commit()
        print("%d rows successfully inserted into the 'puzzle' table.", NUM_PUZZLES)

    except Exception as e:
        print("Error:", e)
    finally:
        if cursor:
            cursor.close()
        if conn:
            conn.close()




def insert_unlock():
    # Connect to the database
    conn = psycopg2.connect(database_url)
    try:
        with conn:
            with conn.cursor() as cursor:
                query = sql.SQL("""
                INSERT INTO "unlock" ("team", "puzzle") VALUES (%s, %s);
                """)
                for team in range(1, NUM_TEAMS+1):
                    for puzzle in range(1, NUM_PUZZLES + 1):
                        if random.random() < 0.3:
                            print(team, puzzle)
                            cursor.execute(query, (team, puzzle))
                            
            conn.commit()
                
    finally:
        conn.close()
        


if __name__ == "__main__":
    import subprocess
    subprocess.run(["diesel","migration","redo" ,"--all"])
    
    users = test_util.prepare_users(NUM_TEAMS * 3)
    insert_random_puzzle()
    insert_unlock()
    
    s = test_util.login(users[0]["id"], users[0]["pw"])
  
    for i in range(NUM_PUZZLES):
        res = s.get(test_util.url + "/decipher_key?puzzle_id={}".format(i))
        print(i, res.text, res)
        
    
