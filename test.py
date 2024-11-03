import requests
import hashlib

url = "https://back-sbojkjgphc.cn-beijing.fcapp.run"
# url = "http://127.0.0.1:9000"


import token_generator

openid = 0x7

pw = hashlib.sha256("1234".encode()).hexdigest()
print(pw)

code = token_generator.get_token(2,4,openid).hex()


res = requests.post(url + "/register", json={
    "username" : "lethe2",
    "password" : pw,
    "token" : code
})
print(res.text)
print("Cookies set by server:", res.cookies)

res = requests.get(url + "/user", cookies= res.cookies)  # 例如：获取用户信息
print("Response for /user:", res.text)






# res = requests.post(url + "/login", json={
#     "username" : "lethe",
#     "password" : "123",
# });

# print(res.text)

# res = requests.post(url + "/login", json={
#     "username" : "lethe",
#     "password" : "1232",
# });

# print(res.text)