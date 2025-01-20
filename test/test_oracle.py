from requests import Session
from test_util import *

from rich.console import Console

console = Console()

default_style = "green"

def print(*args, style=default_style, **kwargs):
    console.print(*args, style=style, **kwargs)

id =  int(input("user_id : "))
pw = input("password: ")

s = login(id, pw)
    
while True:
    order = input().strip()
    
    if order == "create":
        i = int(input("pid:"))
        content = input("content:")
        create_oracle(s, i, content)
        
    elif order  == "reply":
        i = int(input("oid:"))
        r = int(input("refund:"))
        content = input("content:")
        reply_oracle(s, i, r,content)
        
    elif order  == "get":
        i = int(input("oid:"))
        get_oracle(s, i)
        
    elif order  == "check":
        i = int(input("pid:"))
        check_oracle(s, i)
    
    elif order == "list":
        i = int(input("start_oid:"))
        list_oracle(s,i)

    elif order[0] == "quit":
        break
    
    print()
    # info(s)