# SoWasm: an RDF playground based on Sophia

This started as an experiment of compiling [Sophia] into WebAssembly, and grew into a (hopefully) useful playground for RDF validation, conversion, canonicalization, and possibly more in the future...

## Building from source

```bash
wasm-pack build --target web
```

## Testing

Run a local web server (e.g. with `python -m http.server`) and visit <http://localhost:8000/demo/>.


[Sophia]: https://github.com/pchampin/sophia_rs
