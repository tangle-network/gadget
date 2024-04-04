export function webPrompt(message) {
    return prompt(message);
}

export function setConsoleInput() {
    Object.defineProperties(window, {
        startshell: {
            get: function () {
//                 fetch('rs-export.wasm')
//                 .then(response => response.arrayBuffer())
//                 .then(bytes => WebAssembly.instantiate(bytes, {}))
//                 .then(results => {
//                   alert(results.instance.exports.add_one(41));
//                 });
                const importObject = { imports: { } };
                WebAssembly.instantiateStreaming(fetch("rustexport.wasm"), importObject).then(
                  (results) => {
                    alert(results.instance.exports.add_one(41));
                  },
                );
                console.log("Starting web-shell...")
            }
        },
        killshell: {
            get: function () {
                console.log("Killing web-shell...")
            }
        }
    });
    return;
}