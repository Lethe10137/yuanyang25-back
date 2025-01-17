import requests
from test_util import register, login, url, create_team


user_id = register(2343,"234234")
s = login(user_id, "234234")
res = s.post(url + "/create_team")

res = s.get(url + "/info")
print(res.text, res)

res = s.get(url + "/puzzle_status")
print(res.text, res)

res = s.get(url + "/cache_size")
print(res.text, res)
