import hashlib
import requests
from test_util import url


s = requests.Session()
pw = "lethe"
pw = hashlib.sha256(pw.encode()).hexdigest()

token = "ba1e3e6aca01bcbd60d2f1aa27fee74409e77c320d9709ca1e2cfa09013bb4deb8bd630269acfd4da38ab9c5fbd4d491b48190ef5f030873d857fa7afcec8231"

vericode = "95f9 19c5 f8b8 db1d"

res = s.post(url + "/register", json={
    "username" : "lethe2",
    "password" : pw,
    "token" : token
})

print(res.text, res)


res = s.post(url + "/login", json={
    "userid" : 127,
    "auth": {
        "method" : "Totp",
        "data": vericode.replace(" ","")
    }
})
print(res.text)

