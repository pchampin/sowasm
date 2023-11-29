import init, { convert, guess } from "./sowasm.js";
import monacoLoader from 'https://cdn.jsdelivr.net/npm/@monaco-editor/loader@1.4.0/+esm';

function formatToHighlight(format) {
    if (format.endsWith('json')) return 'json'
    if (format.endsWith('xml')) return 'xml'
    return 'sparql'
}

async function main() {
    await init();
    const monaco = await monacoLoader.init();
    // https://github.com/mapbox/mapbox-gl-js/blob/main/debug/standard-style.html#L131

    const guessBox = elt('guess');
    const iformat = elt('iformat');
    const input = elt('input');
    const autoBox = elt('auto');
    const oformat = elt('oformat');
    const output = elt('output');
    const convertBt = elt('convert');
    const url = elt('url');
    const loadBt = elt('load');
    const corsproxyBox = elt('corsproxy');

    let urlSynced = false;
    let guessTimeout = null;
    let convertTimeout = null;

    // Setup the 2 Monaco editors
    const editorsConfig = {
        theme: 'vs-dark',
        language: 'sparql',
        automaticLayout: true,
        scrollBeyondLastLine: false,
        lineNumbers: 'on',
        minimap: {
            enabled: false
        },
        scrollbar: {
            alwaysConsumeMouseWheel: false
        },
    };

    // TODO: add placeholder https://github.com/bultas/monaco-component/blob/master/dist/placeholder.js
    const ieditor = monaco.editor.create(input, {
        ...editorsConfig,
        // value: '(Type or copy some RDF here)',
    });
    const ieditorModel = ieditor.getModel();

    const oeditor = monaco.editor.create(output, {
        ...editorsConfig,
        readOnly: true,
        value: 'Result will appear here',
    });
    const oeditorModel = oeditor.getModel();


    addAllEventListeners();
    applyUrlParams();
    await ensureConsistency();

    /// Function definitions

    function addAllEventListeners() {
        iformat.addEventListener('change', async () => {
            // console.debug("change@iformat");
            guessBox.checked = false;
        });

        guessBox.addEventListener('change', async () => {
            // console.debug("change@guess");
            if (guessBox.checked) {
                await doGuess();
                if (autoBox.checked) {
                    await doConvert();
                }
            }
        });

        input.addEventListener('input', onInputChanged);

        url.addEventListener('input', () => {
            // console.debug("input@url");
            urlSynced= false;
            if (autoBox.checked) {
                doConvertThrottled();
            }
        });

        oformat.addEventListener('change', async () => {
            // console.debug("change@oformat");
            monaco.editor.setModelLanguage(oeditorModel, formatToHighlight(oformat.value));
            await doConvert();
        });

        autoBox.addEventListener('change', async () => {
            // console.debug("change@auto");
            convertBt.disabled = autoBox.checked;
            if (autoBox.checked) {
                await doConvert();
            }
        });

        convertBt.addEventListener('click', doConvert);

        url.addEventListener('input', () => {
            loadBt.disabled = (url.value.length === 0);
        });

        loadBt.addEventListener('click', doLoad);

        elt("permalink").addEventListener('click', () => {
            const urlParams = new URLSearchParams();
            if (guessBox.checked) {
                urlParams.set('guess', '');
            } else {
                urlParams.set('noguess', '');
                urlParams.set('iformat', iformat.value);
            }
            urlParams.set('oformat', oformat.value);
            if (autoBox.checked) {
                urlParams.set('auto', '');
            } else {
                urlParams.set('noauto', '');
            }
            console.log(!input.classList.contains('error'));
            console.log(!input.disabled);
            console.log(!urlSynced);
            console.log(!input.classList.contains('error') && !input.disabled && !urlSynced) ;
            if (!input.classList.contains('error') && !input.disabled && !urlSynced) {
                urlParams.set('input', ieditor.getValue());
            }
            if (url.value) {
                urlParams.set('url', url.value);
            }
            if (corsproxyBox.checked) {
                urlParams.set('corsproxy', '');
            }
            const link = baseUrl();
            link.search = urlParams.toString();
            navigator.clipboard.writeText(link.toString());
        });
    }

    function applyUrlParams() {
        // apply parameters from query string
        const urlParams = new URLSearchParams(window.location.search);
        if (urlParams.has('iformat')) {
            iformat.value = urlParams.get('iformat');
        }
        if (urlParams.has('url')) {
            url.value = urlParams.get('url');
        }
        if (urlParams.has('oformat')) {
            oformat.value = urlParams.get('oformat');
            monaco.editor.setModelLanguage(oeditorModel, formatToHighlight(oformat.value));
        }
        if (urlParams.has('guess')) {
            guessBox.checked = true;
        }
        if (urlParams.has('noguess')) {
            guessBox.checked = false;
        }
        if (urlParams.has('auto')) {
            autoBox.checked = true;
        }
        if (urlParams.has('noauto')) {
            autoBox.checked = false;
        }
        if (urlParams.has('input')) {
            ieditor.setValue(urlParams.get('input'));
        }
        if (urlParams.has('corsproxy')) {
            corsproxyBox.checked = true;
        }
    }

    /// ensure consistency of interface as loaded
    /// (based on URL params or browser-persisted state)
    async function ensureConsistency() {
        // console.debug("ensureConsistency")
        convertBt.disabled = autoBox.checked;
        loadBt.disabled = (url.value.length === 0);
        if (ieditor.getValue()) {
            onInputChanged();
        } else if (url.value) {
            await doLoad();
        }
    }

    function onInputChanged(synced) {
        // console.debug("onInputChanged");
        urlSynced= synced && url.value;
        if (guessBox.checked) {
            doGuessThrottled();
        }
        if (autoBox.checked) {
            doConvertThrottled();
        }
    }

    async function doGuess() {
        // console.debug("doGuess");
        clearTimeout(guessTimeout);
        const guessed = guess(ieditor.getValue());
        iformat.value = guessed;
        monaco.editor.setModelLanguage(ieditorModel, formatToHighlight(guessed));
    }

    function doGuessThrottled() {
        // console.debug("doGuessThrottled");
        clearTimeout(guessTimeout);
        guessTimeout = setTimeout(doGuess, 500);
    }

    async function doConvert() {
        // console.debug("doConvert");
        clearTimeout(convertTimeout);
        output.classList.remove('error');
        try {
            output.disabled = true;
            oeditor.setValue("(parsing)");
            if (!iformat.value) {
                throw "Input format could not be guessed";
            }
            await yieldToBrowser();
            oeditor.setValue(await convert(ieditor.getValue(), iformat.value || null, oformat.value, url.value || null));
        }
        catch (err) {
            output.classList.add('error');
            oeditor.setValue(err);
        }
        finally {
            output.disabled = false;
        }
    }

    function doConvertThrottled() {
        // console.debug("doConvertThrottled");
        clearTimeout(convertTimeout);
        convertTimeout = setTimeout(doConvert, 500);
    }

    async function doLoad() {
        input.classList.remove('error');
        try {
            ieditor.setValue("(loading)");
            ieditor.updateOptions({readOnly: true});
            oeditor.setValue("");
            const resp = await myFetch(url.value);
            if (Math.floor(resp.status / 100) !== 2) {
                throw ("Got status " + resp.status);
            }
            const ctype = resp.headers.get("content-type");
            if (ctype) {
                iformat.value = ctype.split(";")[0];
                // iformat.value will be empty if the ctyp is unknown
                guessBox.checked = (!iformat.value);
            }
            ieditor.setValue(await resp.text())
            onInputChanged(true);
        }
        catch(err) {
            input.classList.add('error');
            ieditor.setValue(err);
        }
        finally {
            input.disabled = false;
        }
    }

    async function myFetch(url, options) {
        options = options || {};
        if (!options.headers) {
            options.headers = {};
        }
        if (!options.headers.accept) {
            options.headers.accept = "application/n-triples,application/n-quads,text/turtle,application/trig,application/ld+json,application/rdf+xml";
        }
        if (corsproxyBox.checked) {
            url = "https://corsproxy.io/?" + encodeURIComponent(url);
            // corsproxy.io seems to request HTML when the user-agent is a browser
            options.headers['user-agent'] = baseUrl();
        }
        return await fetch(url, options);
    }
}

function elt(id) {
    return document.getElementById(id);
}

/// Used solely to yield back control to the browser before continuing
async function yieldToBrowser() {
    return new Promise(function(resolve) {
        setTimeout(resolve, 0);
    });
}

function baseUrl() {
    let url = new URL(window.location);
    url.search = "";
    url.hash = "";
    return url;
}

main();
