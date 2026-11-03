# create-vite-plus

Create a new [Vite+](https://viteplus.dev) project without installing the global `vp` CLI.

```bash
pnpm create vite-plus
npm create vite-plus@latest
yarn create vite-plus
bun create vite-plus
```

This runs [`vp create`](https://viteplus.dev/guide/create) from the `vite-plus` package and uses the package manager that ran it, unless you pass `--package-manager` or create the project inside an existing workspace. Arguments are forwarded unchanged, so the first argument is the template:

```bash
pnpm create vite-plus vite:application --directory my-app
pnpm create vite-plus vite -- --template react-ts
```

npm needs `--` before options meant for `vp create`, for example `npm create vite-plus@latest -- vite:library --no-interactive`.
