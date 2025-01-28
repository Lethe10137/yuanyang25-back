
import token_generator
import time
import requests
import hashlib
import random
import os

url = "http://127.0.0.1:9000"

# try:
#     url = os.environ['SERVER']
# except KeyError:

#     with open(".env", "r") as f:
#         for line in f:
#             try:
#                 name, value = line.strip().split("=")
#                 if(name == "SERVER"):
#                     url = value
#             except:
#                 pass

print(url)


def register(openid: int, raw_pw: str) -> int:
    pw = hashlib.sha256(raw_pw.encode()).hexdigest()
    code = token_generator.get_token(2,0,openid).hex()
    s = requests.session()
    res = s.post(url + "/register", json={
        "username" : "test_{}".format(random.randint(0, 1000000)),
        "password" : pw,
        "token" : code
    })
    uid = res.json()['Success']
    print("User {}, Password {}".format(uid, raw_pw))
    return uid
    
def login(user_id: int, pw: str):
    s = requests.session()
    pw = hashlib.sha256(pw.encode()).hexdigest()
    res = s.post(url + "/login", json={
    "userid" : user_id,
    "auth": {
        "method" : "Password",
        "data": pw
        }
    })
    # print("[{} {}]".format(user_id, pw))
    # print(res.text)
    assert("Success" in res.json())
    return s


def create_team(user_id: int, pw: str) :
    s = login(user_id, pw)
    res = s.post(url + "/create_team")
    # print(res.text)
    res = s.get(url + "/team_veri")
    # print(res.text)

    token = str(res.json()["Success"]["totp"])
    id = int(res.json()["Success"]["id"])
    
    return (s, token, id)
    
def join_team(user_id: int, pw: str, token: str,team_id: int):
    s = login(user_id, pw)
    res = s.post(url + "/join_team", json= {
        "team_id" : team_id,
        "vericode" : token
    }) 
    assert("Success" in res.json())
    return s

def get_decipher_key(s: requests.Session, did: int):
    res = s.get(url + "/decipher_key?decipher_id={}".format(did))
    print(res.text, res)
    
    try:
        return res.json()["Success"]
    except:
        return ""

def buy_decipher_key(s: requests.Session, did: int):
    res = s.post(url + "/unlock?decipher_id={}".format(did))
    print(res.text, res)
    
    try:
        return res.json()["Success"]["key"]
    except:
        try:
            return res.json()["AlreadyUnlocked"]
        except:
            return ""
        

def submit_answer(s: requests.Session, pid: int, cipher: str, answer: str):
    sha = hashlib.sha256((cipher + answer).encode()).hexdigest()
    print(sha)

    res = s.post(
        url + "/submit_answer", json= {
            "puzzle_id" : pid,
            "answer" : sha
        }
    )
    print(res.text, res)
    try:
        return res.json()["Success"]["key"]
    except:
        return ""
        

def info(s: requests.Session):
    res = s.get(url + "/info")
    print(res.text, res)
    res = s.get(url + "/puzzle_status")
    print(res.text, res)
    res = s.get(url + "/rank")
    print(res.text, res)
    

def prepare_users(user_cnt):
    users = []
    
    token = ""
    team_id = -1
    
    for i in range(user_cnt):
        pw = "30240184pw_{}".format(i+1)
        user_id =register(i, pw)
        users.append({
            "openid" : i,
            "pw" : pw,
            "id" : user_id
        })
        
        if(i % 3 == 0):
            _, new_token, new_team_id = create_team(user_id, pw)
            token = new_token
            team_id = new_team_id
        else:
            try:
                join_team(user_id, pw, token, team_id)
            except:
                pass
    
    return users

        
def create_oracle(s: requests.Session, puzzle_id: int,content: str):
    res = s.post(
        url + "/create_oracle", json= {
            "puzzle_id" : puzzle_id,
            "content" : content
        }
    )
    print(res.text, res)
    
def reply_oracle(s: requests.Session, oracle_id: int,refund:int,content: str):
    res = s.post(
        url + "/staff_reply_oracle", json= {
            "oracle_id" : oracle_id,
            "refund_amount": refund,
            "content" : content
        }
    )
    print(res.text, res)

def get_oracle(s: requests.Session, oracle_id: int):
    res = s.get(
        url + "/get_oracle?oracle_id={}".format(oracle_id), 
    )
    print(res.text, res)

def check_oracle(s: requests.Session, puzzle_id: int):
    res = s.get(
        url + "/check_oracle?puzzle_id={}".format(puzzle_id), 
    )
    print(res.text, res)

def list_oracle(s: requests.Session, start_id: int):
    res = s.get(
        url + "/staff_list_oracle?start_oracle_id={}&limit=10".format(start_id), 
    )
    print(res.text, res)


def register_staff(openid, pw, username):
    pw = hashlib.sha256(pw.encode()).hexdigest()
    code = token_generator.get_token(2,4,openid).hex()
    s = requests.session()
    res = s.post(url + "/register", json={
        "username" : username,
        "password" : pw,
        "token" : code
    })
    
    uid = res.json()['Success']
    print("Id {} \t Staff {}".format( uid, username ))
    return uid

if __name__ == "__main__":
    prepare_users(15)
    
    with open("test/example_data/staff.txt", "r") as f:
        for line in f:
            line = line.replace("\t", " ")
            line = line.split()
            if len(line) == 2:
                register_staff(int(line[1], 16), "9787534XnJiKsWh", line[0])
    
    