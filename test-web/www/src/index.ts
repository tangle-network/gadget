import './index.css';

import * as we from 'wasm_examples';
import { memory } from 'wasm_examples/wasm_examples_bg.wasm';
import $ from 'cash-dom';
import init, { web_main, TomlConfig, Opt } from 'web-shell';
// import * as shell from "web-shell";


$(async () => {
    // Initialize our Wasm package
    we.run();
    await init();
//     shell.initSync();

    $('#fileSelect').on('click', () => we.sayHello());

    $('#sendRustStructs').on('click', async () => {
        console.log("Starting Rust Web Main Function");

        const config: TomlConfig = {
            bind_ip: "0.0.0.0",
            bind_port: 8081,
            url: "https://github.com/webb-tools/gadget",
            bootnodes: ["/ip4/127.0.0.1/udp/1234", "foo", "/ip4/127.0.0.1/udp/1235","/ip4/127.0.0.1/udp/1236"],
            node_key: "0000000000000000000000000000000000000000000000000000000000000001",
            base_path: "test/path/to/",
            keystore_password: null,
            chain: "local_testnet",
        };

        const options: Opt = {
            config: null,
            verbose: 0,
            pretty: false,
            options: config,
        };

        var keys: string[] = ["0000000000000000000000000000000000000000000000000000000000000001", "0000000000000000000000000000000000000000000000000000000000000002"]


        const message = await web_main(config, options, keys);
        console.log(message);

        console.log("Ending Rust Web Main Function");

    });


    //#region Importing/exporting functions from JS/Rust
    // Call functions exported from Rust
    $('#sayHello').on('click', () => we.sayHello());

    // Define a global function that will be called by Rust
    (<any>window).myAlert = (msg: string) => alert(msg);
    $('#sayHelloWithMyAlert').on('click', () => we.sayHelloWithMyAlert());
    //#endregion

    //#region Handle parameters
    // Demonstrate working with numbers
    $('#workWithNumbers').on('click', () => {
        console.log(`The result is ${we.workWithNumbers(0x1ffffffffffffd, 1, 1).toString(16)}`);
        console.log(`The result is ${we.workWithNumbers(0x1ffffffffffffe, undefined, 1).toString(16)}`);
    });

    $('#bools').on('click', () => alert((we.allTrue(true, true) ? "" : "NOT ") + "all are true"));
    //#endregion

    //#region Panic
    $('#panic').on('click', () => {
        try {
            we.panic();
        } catch (ex) {
            console.error(ex);
        }
    });
    //#endregion

    //#region Number slices
    $('#fib').on('click', () => {
        const buffer = new Int32Array(10);
        we.fillWithFibonacci(buffer);
        $('#fibResult').text(buffer.join(','));
    });
    //#endregion

    //#region Strings
    $('#strings').on('click', () => {
        // Note that the following text contains a paired surrogate char (\ud834\udd1e = 𝄞).
        // It also contains an unpaired surrogate char (\ud834). This is fine in JavaScript.
        // If you pass it to Rust, Rust will replace the unpaired surrogate with \ufffd (�).
        let text = "ABCabc🎵\ud834\udd1e\ud834";

        // Print the text to console and pass-through the text from Rust back to JS.
        // Passing through the text will "destroy" the unpaired surrogate.
        let newText = we.writeToConsole("Original: " + text);

        // It is fine to "fix" the unpaired surrogate based on the JS string.
        // Fixing it based on the Rust string will not work because unpaired surrogate
        // will have been "destroyed".
        text += "\udd1e";
        newText += "\udd1e";
        we.writeToConsole("Fixed (JS): " + text);
        we.writeToConsole("\"Fixed\" (Rust): " + newText);

        // For more information about handling of unpaired surrogates read
        // https://rustwasm.github.io/docs/wasm-bindgen/reference/types/str.html#utf-16-vs-utf-8
    });
    //#endregion

    //#region Exported Types
    $('#exportedType').on('click', () => {
        let person: we.Person, p: we.Person;
        try {
            // Create person using constructor function.
            // Note that person lives in "Wasm land".
            person = we.Person.new('JS', 'Foo', 42);

            // Call method. Method takes ownership of person and will
            // therefore free the associated memory.
            p = we.fixAge(person, 2);
            console.log(`${p.first_name}, ${p.last_name}, ${p.age}`);
        } catch (ex) {
            console.error(ex);
        } finally {
            // Free person returned by fixAge.
            if (p) p.free();
        }
    });
    //#endregion

    //#region Working with JsValue
    $('#jsValue').on('click', () => {
        try {
            // Tip: Try to mess up person object and see error handling in action
            const p = we.fixAgeDynamic({ firstName: 'Foo', lastName: 'Bar', age: 42}, 2);
            console.log(p);
        } catch (ex) {
            console.error(ex);
        }
    });
    //#endregion

    //#region Working with serde_wasm_bindgen
    $('#serdeWasm').on('click', () => {
        try {
            // Tip: Try to mess up person object and see error handling in action
            const p = we.fixAgeSerdeWasm({ firstName: 'Foo', lastName: 'Bar', age: 42}, 2);
            console.log(p);
        } catch (ex) {
            console.error(ex);
        }
    });
    //#endregion

//     //#region Working with shared buffers
//     // Create a new display object
//     const display = we.Display.new(30);
//
//     // Get shared buffer and access it via Uint8Array
//     const pixel =  new Uint8Array(memory.buffer, display.pixel(), 30);
//
//     // Regularly redraw canvas
//     const canvas = $('#canvas')[0] as HTMLCanvasElement;
//     const ctx = canvas.getContext('2d', { alpha: false });
//     setInterval(() => {
//         ctx.clearRect(0, 0, canvas.width, canvas.height);
//         for (let i = 0; i < pixel.length; i++) {
//             if (pixel[i] > 0) {
//                 // Intensity of RED depends on value in shared buffer
//                 ctx.fillStyle = `hsl(0, 100%, ${100 - 10 * pixel[i]}%)`;
//                 ctx.fillRect(i * 10, 0, 10, 10);
//             }
//         }
//
//         display.next();
//     }, 50);
//     //#endregion

    // Setup todo maintenance (done 100% in Rust)
    we.setup_todo();
});