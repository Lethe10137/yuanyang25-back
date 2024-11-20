import requests
import hashlib

# url = "https://back-sbojkjgphc.cn-beijing.fcapp.run"
url = "http://127.0.0.1:9000"


import token_generator
import time

openid = time.time_ns()
pw = "12348129371987298"
pw = hashlib.sha256(pw.encode()).hexdigest()
code = token_generator.get_token(2,4,openid).hex()
s = requests.session()


# Normal register
res = s.post(url + "/register", json={
    "username" : "lethe2",
    "password" : pw,
    "token" : code
})

res = s.get(url + "/user") 
print("Response for /user:", res.text)

assert(res.text.startswith("Privilege"))

user_id = int(res.text.split(" ")[-1])

s = requests.Session()

print("\n+++++++++++ New Session ++++++++++")

res = s.get(url + "/user")  
print("Response for /user:", res.text)
assert(res.text.startswith("No user"))

#use password to verificate, expected to fail!
res = s.post(url + "/login", json={
    "userid" : user_id,
    "auth": {
        "method" : "Password",
        "data": pw[:-1] + "k"
    }
})

print(res.text)
assert(res.json() == "Error")

#use password to verificate, expected to success!
res = s.post(url + "/login", json={
    "userid" : user_id,
    "auth": {
        "method" : "Password",
        "data": pw
    }
})

print(res.text)
assert("Success" in res.json())

res = s.get(url + "/user")  
print("Response for /user:", res.text)
assert(res.text.startswith("Privilege"))

print("\n+++++++++++ New Session ++++++++++")

s = requests.Session()

res = s.get(url + "/user")  
print("Response for /user:", res.text)
assert(res.text.startswith("No user"))

res = s.post(url + "/login", json={
    "userid" : user_id,
    "auth": {
        "method" : "Totp",
        "data": "083ab3d834"
    }
})
print(res.text)
assert(res.status_code >= 400)

vericode = token_generator.vericode(f"{openid:042x}")

print("verification code:", vericode)

res = s.post(url + "/login", json={
    "userid" : user_id,
    "auth": {
        "method" : "Totp",
        "data": vericode
    }
})
print(res.text)
assert("Success" in res.json())

res = s.get(url + "/user")  # 例如：获取用户信息
print("Response for /user:", res.text)

assert(res.text.startswith("Privilege"))

res = s.post(url + "/create_team")
print(res.text)
assert("Success" in res.json())

res = s.post(url + "/create_team")
print(res.text)
assert("AlreadyInTeam" in res.json())




