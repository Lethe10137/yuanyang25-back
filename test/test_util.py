
import token_generator
import time
import requests
import hashlib
import random

url = "http://127.0.0.1:9000"


def register(openid: int, pw: str) -> int:
    pw = hashlib.sha256(pw.encode()).hexdigest()
    code = token_generator.get_token(2,4,openid).hex()
    s = requests.session()
    res = s.post(url + "/register", json={
        "username" : "test_{}".format(random.randint(0, 1000000)),
        "password" : pw,
        "token" : code
    })
    print(res.text)
    return res.json()['Success']
    
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
    print("[{} {}]".format(user_id, pw))
    print(res.text)
    assert("Success" in res.json())
    return s


def create_team(user_id: int, pw: str) :
    s = login(user_id, pw)
    res = s.post(url + "/create_team")
    print(res.text)
    res = s.get(url + "/team_veri")
    print(res.text)

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


def prepare_users(user_cnt):
    users = []
    
    token = ""
    team_id = -1
    
    for i in range(user_cnt):
        pw = random.randbytes(4).hex()
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

        

