# command_view_yarn4

## `vp pm view testnpm2`

should view testnpm2 package information (uses yarn npm info)

```
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
  directories: {}
}
```

## `vp pm view testnpm2 version`

should view testnpm2 version field (uses yarn npm info)

```
{
  name: 'testnpm2',
  version: '1.0.1'
}
```
