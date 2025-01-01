import random
import string
import psycopg2
from psycopg2 import sql
from psycopg2.extras import execute_values

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
NUM_USERS = 20
MAX_TEAM_SIZE = 5

def generate_teams(num_teams):
    teams = []
    for _ in range(num_teams):
        team = {
            "is_staff": random.choice([True, False]),
            "token_balance": random.randint(0, 1000),
            "confirmed": random.choice([True, False]),
            "max_size": MAX_TEAM_SIZE,
            "size": 0,  # Start with 0 members
            "salt": os.urandom(32).hex(),
        }
        teams.append(team)
    return teams

def generate_users(num_users, team_ids):
    users = []
    for _ in range(num_users):
        team = random.choice(team_ids + [None])  # Some users might not belong to a team
        user = {
            "openid": os.urandom(16).hex(),
            "team": team,
            "username": f"user_{os.urandom(4).hex()}",
            "password": os.urandom(16).hex(),
            "salt": os.urandom(16).hex(),
            "privilege": random.randint(0, 5),
        }
        users.append(user)
    return users

def insert_teams(cursor, teams):
    query = """
        INSERT INTO team (is_staff, token_balance, confirmed, max_size, size, salt)
        VALUES %s RETURNING id
    """
    values = [
        (
            team["is_staff"],
            team["token_balance"],
            team["confirmed"],
            team["max_size"],
            team["size"],
            team["salt"],
        )
        for team in teams
    ]
    execute_values(cursor, query, values)
    return [row[0] for row in cursor.fetchall()]

def insert_users(cursor, users):
    query = """
        INSERT INTO users (openid, team, username, password, salt, privilege)
        VALUES %s
    """
    values = [
        (
            user["openid"],
            user["team"],
            user["username"],
            user["password"],
            user["salt"],
            user["privilege"],
        )
        for user in users
    ]
    execute_values(cursor, query, values)
    

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
            INSERT INTO puzzle (bounty, title, answer, key, content)
            VALUES (%s, %s, %s, %s, %s)
        """)


        for _ in range(NUM_PUZZLES):
            bounty = random.randint(1, 1000)  # Random integer for bounty
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

def insert_users_teams():
    # Connect to the database
    conn = psycopg2.connect(database_url)
    try:
        with conn:
            with conn.cursor() as cursor:
                # Generate and insert teams
                teams = generate_teams(NUM_TEAMS)
                team_ids = insert_teams(cursor, teams)

                # Generate and insert users
                users = generate_users(NUM_USERS, team_ids)
                insert_users(cursor, users)

                print(f"Inserted {len(teams)} teams and {len(users)} users.")
    finally:
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
    insert_random_puzzle()
    insert_users_teams()
    insert_unlock()
    
    
    
