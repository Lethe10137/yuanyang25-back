import hashlib
import random
from random import SystemRandom
import time

import os
from pathlib import Path

def load_env(filepath: str = ".env"):
    env_path = Path(filepath)
    if not env_path.exists():
        print(f"{filepath} 文件不存在")
        return
    
    with open(env_path) as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                key, value = line.split("=", 1)
                os.environ[key] = value.strip().strip('"').strip("'")

load_env()


# version: 8 bits
# openid: ￼28 * 6 = 168 bits
# time: in minutes, 32bit￼
# mark: 8bits
# nonce: ￼40bits
# Hash: SHA256 256


salt = os.environ["REGISTER_TOKEN"]

# print(salt)

def genenrate_openid():
    c = SystemRandom()
    return c.getrandbits(28 * 6)


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
    
token = get_token(2, 4, genenrate_openid())

# print(token.hex())

# raw  028312ec8e8bdd45bc29818a2a83ddd009a15107243901b81742042f47e6428f
# hash a364690f58d89c90fd7843b3ca306c27f5f62a9ea024c1b4aed4fc74fc686471

verify_token = os.environ["VERIFY_TOKEN"]


#openid: lower case hex humber, which length is 168 bits or 42 hexadecimal digits
def vericode(very_session: str, openid: str) -> str:
    hash = hashlib.sha512(very_session.encode())
    hash.update(openid.encode())
    hash.update(verify_token.encode())
    hash = hash.digest()
    
    return hash.hex()[:8]