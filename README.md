# SoWasm: an RDF playground based on Sophia

This started as an experiment of compiling [Sophia] into WebAssembly, and grew into a (hopefully) useful playground for RDF validation, conversion, canonicalization, and possibly more in the future...

## Building from source

```bash
wasm-pack build --target web
```

## Running locally

First run the command above to build the wasm package.

Install dependencies:

```sh
cd demo
npm i
```

Run a local web server on <http://localhost:8000/>.

```sh
npm run dev
```

Build for production:

```sh
npm run build
```

Deploy production build:

```sh
npm run preview
```

[Sophia]: https://github.com/pchampin/sophia_rs
