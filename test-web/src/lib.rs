mod utils;

use wasm_bindgen::prelude::*;

use wasm_bindgen::{prelude::*};
use serde::{Serialize, Deserialize};
use web_sys::{HtmlButtonElement, HtmlInputElement, HtmlLiElement, HtmlUListElement};
use wasm_bindgen::JsCast;

#[wasm_bindgen(start)]
pub fn run() {

    console_error_panic_hook::set_once();

    log("🦀 Rust Wasm initialized 🤘".to_string());
}


#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);      // Import built-in alert function
fn myAlert(s: &str);    // Import global custom function
#[wasm_bindgen(js_namespace = console)]
fn log(s: String);        // Import console.log function
}



/// Says hello using JavaScript's `alert` function
#[wasm_bindgen(js_name = sayHello)]
pub fn say_hello() {
    alert("Hello World!");
}

/// Says hello using custom `myAlert` function
#[wasm_bindgen(js_name = sayHelloWithMyAlert)]
pub fn say_hello_with_my_alert() {
    myAlert("Hello World!");
}


/// Adds three numbers
///
/// * `x` - A float number
/// * `y` - An optional float number
/// * `r` - An int number
#[wasm_bindgen(js_name = workWithNumbers)]
pub fn work_with_numbers(x: f64, y: Option<f32>, r: i32) -> i64 {
    x as i64 + match y { Some(vy) => vy as i64, _ => 0i64 } + r as i64
}


/// Checks whether all parameters are true
#[wasm_bindgen(js_name = allTrue)]
pub fn all_true(a: bool, another: bool) -> bool {
    a && another
}


/// Will panic
#[wasm_bindgen]
#[allow(unconditional_panic)]
pub fn panic() -> i32 {
    let x = 42;
    let y = 0;
    x / y
}


/// Fills given number slice with fibonacci numbers
#[wasm_bindgen(js_name = fillWithFibonacci)]
pub fn fill_with_fibonacci(buffer: &mut [i32]) {
    if buffer.is_empty() { return; }
    buffer[0] = 0;

    if buffer.len() == 1 { return; }
    buffer[1] = 1;

    for i in 2..buffer.len() {
        buffer[i] = buffer[i - 1] + buffer[ i - 2];
    }
}


/// Prints the given message to console and returns the message unchanged.
#[wasm_bindgen(js_name = writeToConsole)]
pub fn write_to_console(msg: &str) -> String {
    log(msg.to_string());
    msg.to_string()
}


/// Represents a person
#[wasm_bindgen]
pub struct Person {
    first_name: String,
    last_name: String,
    pub age: f64,
}

#[wasm_bindgen]
impl Person {
    /// Constructs a new person.
    #[wasm_bindgen]
    pub fn new(first_name: String, last_name: String, age: f64) -> Person {
        Person { first_name, last_name, age}
    }

    #[wasm_bindgen(getter)]
    pub fn first_name(&self) -> String {
        self.first_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn last_name(&self) -> String {
        self.last_name.clone()
    }
}

impl Drop for Person {
    fn drop(&mut self) {
        log(format!("Dropping {}", self.first_name));
    }
}

/// Adds given change to a person's age and returns the changed person.
#[wasm_bindgen(js_name = "fixAge")]
pub fn fix_age(person: Person, age_change: f64) -> Person {
    Person { first_name: "Rust".to_string(), last_name: person.last_name.clone(), age: person.age + age_change }
}

#[derive(Serialize, Deserialize)]
struct PersonDynamic {
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "lastName")]
    last_name: String,
    age: f64,
}

fn into_js_error(err: impl std::error::Error) -> JsValue {
    js_sys::Error::new(&err.to_string()).into()
}

#[wasm_bindgen(js_name = "fixAgeDynamic")]
pub fn fix_age_dynamic(person: JsValue, age_change: JsValue) -> Result<JsValue, JsValue> {
    let mut p: PersonDynamic = serde_wasm_bindgen::from_value(person).map_err(into_js_error)?;

    if let Some(ac) = age_change.as_f64() {
        p.age += ac;
    }

    let result = serde_wasm_bindgen::to_value(&p).map_err(into_js_error)?;
    Ok(result)
}

#[wasm_bindgen(js_name = "fixAgeSerdeWasm")]
pub fn fix_age_serde_wasm(person: JsValue, age_change: JsValue) -> Result<JsValue, JsValue> {
    let mut p: PersonDynamic = serde_wasm_bindgen::from_value(person)?;
    let ac: f64 = serde_wasm_bindgen::from_value(age_change)?;
    p.age += ac;
    Ok(serde_wasm_bindgen::to_value(&p)?)
}

#[derive(PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

#[wasm_bindgen]
pub struct Display {
    pixel: Vec<u8>,
    direction: Direction,
}

/// Intensity of leading light
const MAX_INTENSITY: u8 = 5;

#[wasm_bindgen]
impl Display {
    /// Creates a new "display". Each display represents the light intensity of the corresponding LED.
    pub fn new(size: usize) -> Display {
        let mut result = Display { pixel: vec![0; size], direction: Direction::Right, };
        result.pixel[0] = MAX_INTENSITY;
        result
    }

    /// Gets a pointer to the shared buffer with light intensities per LED.
    pub fn pixel(&self) -> *const u8 {
        self.pixel.as_ptr()
    }

    /// Advance LED to next position.
    pub fn next(&mut self) {
        let ix = self.pixel.iter().position(|p| *p == MAX_INTENSITY).unwrap();
        let next_ix: usize;
        if ix == 0 {
            self.direction = Direction::Right;
        }
        else if ix == self.pixel.len() - 1 {
            self.direction = Direction::Left;
        }

        next_ix = ix + match self.direction { Direction::Left => -1, Direction::Right => 1 } as usize;

        self.pixel.iter_mut().for_each(|p| if *p > 0 { *p -= 1; });

        self.pixel[next_ix] = MAX_INTENSITY;
    }
}

#[wasm_bindgen]
pub fn setup_todo() {
    let mut todo_items = Vec::<String>::new();

    let window = web_sys::window().unwrap();

    let document = window.document().unwrap();
    let new_todo_item_element = document.get_element_by_id("newTodoItem").unwrap();
    let todo_items_element = document.get_element_by_id("todoItems").unwrap();

    let btn_handler = Closure::wrap(Box::new(move || {

        let new_todo_item = new_todo_item_element.dyn_ref::<HtmlInputElement>().unwrap();
        let todo_items_list = todo_items_element.dyn_ref::<HtmlUListElement>().unwrap();

        todo_items.push(new_todo_item.value());

        todo_items_list.set_inner_text("");
        for item in todo_items.iter() {
            let new_li_element = document.create_element("li").unwrap();
            let new_li = new_li_element.dyn_ref::<HtmlLiElement>().unwrap();

            new_li.set_inner_text(item.as_str());

            todo_items_list.append_child(new_li).unwrap();
        }

    }) as Box<dyn FnMut()>);

    window.document().unwrap().get_element_by_id("addTodoItem").unwrap().dyn_ref::<HtmlButtonElement>().unwrap()
        .set_onclick(Some(btn_handler.as_ref().unchecked_ref()));

    btn_handler.forget();
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, wasm-pack-template!");
}
