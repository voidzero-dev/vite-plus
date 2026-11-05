import hashlib,json,os,pathlib
root=pathlib.Path(os.environ['JOB_ENV_DIR'])
receipt=json.loads((root/'setup-receipt.json').read_text())
for name,expected in receipt['hashes'].items():
    assert hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest()==expected,name
print('Setup artifacts verified')
