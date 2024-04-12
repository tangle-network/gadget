//import * as wasm from "hello-wasm-pack";
//import * as web from './../../web-shell/pkg/web_shell.js';
import init, { web_main, TomlConfig, SupportedChains, Opt } from "web-shell";

await init();

//wasm.greet();

const openFileButton = document.getElementById("fileButton");

async function askUserForFiles(multiple = false) {
  const input = document.createElement("input");
  input.type = "file";
  input.multiple = multiple;
  input.click();

  return new Promise((resolve) => {
    input.addEventListener("change", event => resolve(input.files));
  });
}

async function parseJsonFile(file) {
  return new Promise((resolve, reject) => {
    const fileReader = new FileReader()
    fileReader.onload = event => resolve(JSON.parse(event.target.result))
    fileReader.onerror = error => reject(error)
    fileReader.readAsText(file)
  })
}

openFileButton.addEventListener("click", async event => {
  try {
    const inFile = await askUserForFiles(false);
    const jsonFile = inFile[0];
    var ext = jsonFile.name.split(".").pop().toLowerCase();
    if (ext != "json") {
        alert('Upload json file');
        return false;
    }
    if (jsonFile != undefined) {
        const object = await parseJsonFile(jsonFile);
        console.log(object.keyOne);
        console.log(object.keyTwo);
        console.log(object.keyThree);
    }
    await startWebMain();
  }
  catch (e) {
    alert("Error Occurred.");
    console.log(e);
  }
});

function getConfigs() {
    const bind_ip = "0.0.0.0";
    const bind_port = "30556";
    const chain = "local_testnet";
    node_key = "0000000000000000000000000000000000000000000000000000000000000001";
}

async function startWebMain() {
    console.log("Starting Rust Web Main Function");

    const config: TomlConfig = {
        bind_ip?: "0.0.0.0",
        bind_port?: 8081,
        url?: "localhost",
        bootnodes?: ["0.0.0.1","0.0.0.2","0.0.0.3"],
        base_path: "test/path/to/",
        keystore_password: null,
        chain?: "local_testnet",
    }

    const options: Opt = {
        config: null,
        verbose: 0,
        pretty: false,
        options: config,
    }


    await web_main(config, options);

    console.log("Ending Rust Web Main Function");
//    const importObject = { module: { } };
//    WebAssembly.instantiateStreaming(fetch("web_shell_bg.wasm"), importObject).then(
//        (results) => {
////            await results.instance.exports.web_main();
//            alert("Successfully awaited web_main");
//            console.log(results);
//        },
//    );
//    fetch('web_shell_bg.wasm')
//    .then(response => {
//        console.log("response checkpoint");
//        console.log(response.type);
//        response.arrayBuffer()
//    })
//    .then(bytes => {
//        console.log("bytes checkpoint");
//        WebAssembly.instantiate(bytes, {})
//    })
//    .then(async (results) => {
//        console.log("results checkpoint");
//        console.log(results);
////        await results.instance.exports.web_main();
//        alert("Successfully awaited web_main");
//    })
//    .catch(error => {
//        alert("Error Occurred.");
//        console.log(error);
//    });
}

//bind_ip = "0.0.0.0"
//bind_port = 30556
//bootnodes = []
//# 12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
//node_key = "0000000000000000000000000000000000000000000000000000000000000001"
//chain = "local_testnet"
//base_path = "../tangle/tmp/bob"