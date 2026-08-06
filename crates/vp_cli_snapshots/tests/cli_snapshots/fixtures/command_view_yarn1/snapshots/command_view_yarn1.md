# command_view_yarn1

## `vp pm view testnpm2`

should view testnpm2 package information (uses yarn info)

```
yarn info <version>
warning package.json: No license field
{
  name: 'testnpm2',
  time: {
    modified: '2022-06-27T05:33:14.474Z',
    created: '2015-07-18T03:01:26.187Z',
    '0.0.1': '2015-07-18T03:01:26.187Z',
    '0.0.2': '2015-07-18T03:05:15.290Z',
    '0.0.3': '2015-07-18T03:13:59.614Z',
    '0.0.4': '2015-07-18T03:27:34.134Z',
    '0.0.5': '2015-07-18T03:34:04.517Z',
    '0.0.6': '2015-07-18T03:44:19.300Z',
    '0.0.7': '2015-07-18T03:58:01.493Z',
    '0.0.8': '2015-07-18T04:07:17.636Z',
    '1.0.0': '2015-07-18T18:23:11.382Z',
    '1.0.1': '2015-07-18T18:23:59.560Z',
    '2.0.0': '2018-04-08T09:30:53.747Z',
    '2.0.1': '2018-04-08T09:35:07.572Z'
  },
  maintainers: [
    {
      name: 'fengmk2',
      email: 'fengmk2@gmail.com'
    }
  ],
  'dist-tags': {
    latest: '1.0.1',
    'release-1': '1.0.1'
  },
  versions: [
    '1.0.0',
    '1.0.1'
  ],
  license: 'ISC',
  version: '1.0.1',
  main: 'index.js',
  scripts: {
    test: 'echo "Error: no test specified" && exit 1'
  },
  dist: {
    shasum: '8c7b209a673c360e540ab2777242171fd30fdee9',
    tarball: 'https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz',
    integrity: 'sha512-F4AQ+KmzhbOSlt7ae+X2O8IJktFZAcN6OK169TT4ny7M3e4Vje7NITZTOU31AtEk9L/Z8lrCrqinl/eY6WPuEw==',
    signatures: [
      {
        keyid: 'SHA256:jl3bwswu80PjjokCgh0o2w5c2U4LhQAE57gj9cz1kzA',
        sig: 'MEQCICqyUi6OO0qltJG0Z2fI021Q87C6zFIWH9h2lb9PsyRKAiAHU26fIlW7Om8JPh2BEx72YAAVP2yXS2bvf9vzc/yjaw=='
      }
    ]
  },
  directories: {}
}

Done in <duration>.
```

## `vp pm view testnpm2 version`

should view testnpm2 version field (uses yarn info)

```
yarn info <version>
warning package.json: No license field
1.0.1

Done in <duration>.
```
