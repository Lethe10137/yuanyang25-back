import requests
import hashlib
from random import SystemRandom
import time


# 测试环境使用
salt = "oUGdVSiysbr72t1eobDMxcY9MTRgtNB6as475eSQitA9FjOS4wkdeIJq4MnDmCJERYsotfagq7AWp99fdySNV5gpgVrNw0xiq3HDGmENiIArRtuNSL9TDW3odoqszrQL"


def get_time():
    t = int(time.time() / 60) & 0xffffffff
    return t
    
def get_nonce():
    c = SystemRandom()
    return c.getrandbits(40)

def get_token(version, mark, openid):
    version = version & 0xff
    mark = mark & 0xff
    
    token = version
    
    token <<= 168
    token |= openid # 168bits
    
    token <<= 32
    token |= get_time() # 32bits
    
    token <<= 8
    token |= mark #8bits
    
    token <<= 40
    token |= get_nonce() #40bits
    
    byte_array = token.to_bytes(32, byteorder='big', signed=False)
    
    
    # print("raw ", byte_array.hex())
    
    hash = hashlib.sha256(byte_array)
    hash.update(salt.encode("utf-8"))
    hash = hash.digest()
    
    # print("hash", hash.hex())
    
    result =bytes(a ^ b for a, b in zip(byte_array, hash)) + hash
 
    return result


if __name__ == "__main__":
    
    url = "http://127.0.0.1:9000"

    openid = 19260817
    pw ="kk2342"

    pw = hashlib.sha256(pw.encode()).hexdigest()

    
    s = requests.session()


    res = s.post(url + "/register", json={
        "username" : "lethe2",
        "password" : pw,
        "token" : get_token(2,0,openid).hex()
    })

    print("Response for /register:", res.text)
