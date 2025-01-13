from requests import Session
from test_util import *

from rich.console import Console

console = Console()

default_style = "green"

def print(*args, style=default_style, **kwargs):
    console.print(*args, style=style, **kwargs)


s = login(1, "pw1")
key = ""
    
while True:
    order = input()
    order = order.split()
    
    if order[0] == "get":
        did = int(order[1])
        print("getting decipher key for ", did)
        key = get_decipher_key(s, did) + key
    elif order[0] == "buy":
        did = int(order[1])
        print("buying decipher key for ", did)
        key = buy_decipher_key(s, did) + key
    elif order[0] == "ans":
        pid = int(order[1])
        cipher = order[2]
        if cipher == "key":
            cipher = key
        answer = order[3]
        print("submiting answer ", pid)
        key = submit_answer(s, pid, cipher, answer) + key
    elif order[0] == "quit":
        break
    
    key = key[:64]
    
    print("Current Key, ", key, style="blue")
    
    info(s)