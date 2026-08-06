# command_pack_npm10

## `vp pm pack --json`

should pack current package

```
[
  {
    "id": "command-pack-npm10@1.0.0",
    "name": "command-pack-npm10",
    "version": "1.0.0",
    "size": 220,
    "unpackedSize": 206,
    "shasum": "0cf1c1fd651186224e8470e33c869588da49af76",
    "integrity": "sha512-2kQo+GXFturmXYEWUkUpr2LN0InQu70ecQiMHBKh59yQ4C4tduWyCdZ6uVoIJ2dXiLNQN9f6TFaWGmUe7bwVvQ==",
    "filename": "command-pack-npm10-1.0.0.tgz",
    "files": [
      {
        "path": "package.json",
        "size": 206,
        "mode": 420
      }
    ],
    "entryCount": 1,
    "bundled": []
  }
]
```

## `vpt rm -f command-pack-npm10-1.0.0.tgz`


## `vp pm pack --pack-destination ./dist --json`

should pack with destination

```
[
  {
    "id": "command-pack-npm10@1.0.0",
    "name": "command-pack-npm10",
    "version": "1.0.0",
    "size": 220,
    "unpackedSize": 206,
    "shasum": "0cf1c1fd651186224e8470e33c869588da49af76",
    "integrity": "sha512-2kQo+GXFturmXYEWUkUpr2LN0InQu70ecQiMHBKh59yQ4C4tduWyCdZ6uVoIJ2dXiLNQN9f6TFaWGmUe7bwVvQ==",
    "filename": "command-pack-npm10-1.0.0.tgz",
    "files": [
      {
        "path": "package.json",
        "size": 206,
        "mode": 420
      }
    ],
    "entryCount": 1,
    "bundled": []
  }
]
```

## `vpt rm -rf ./dist`

```
```

## `vp pm pack --json -- --loglevel=warn`

should support pass through arguments

```
[
  {
    "id": "command-pack-npm10@1.0.0",
    "name": "command-pack-npm10",
    "version": "1.0.0",
    "size": 220,
    "unpackedSize": 206,
    "shasum": "0cf1c1fd651186224e8470e33c869588da49af76",
    "integrity": "sha512-2kQo+GXFturmXYEWUkUpr2LN0InQu70ecQiMHBKh59yQ4C4tduWyCdZ6uVoIJ2dXiLNQN9f6TFaWGmUe7bwVvQ==",
    "filename": "command-pack-npm10-1.0.0.tgz",
    "files": [
      {
        "path": "package.json",
        "size": 206,
        "mode": 420
      }
    ],
    "entryCount": 1,
    "bundled": []
  }
]
```

## `vpt rm -f command-pack-npm10-1.0.0.tgz`

