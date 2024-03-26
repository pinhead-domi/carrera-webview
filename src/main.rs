use async_stream::stream;
use axum::{
  extract::State,
  http::StatusCode,
  response::sse::{Event, KeepAlive, Sse},
  routing::{get, post},
  Json, Router,
};

use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
//use serialport::{available_ports, SerialPort};
use tokio_serial::{available_ports, SerialPortBuilderExt, SerialStream};
//use tokio_serial::SerialPort;
use std::{borrow::BorrowMut, path::PathBuf, time::SystemTime};
use std::{convert::Infallible, time::Duration};
use std::{
  sync::{Arc, Mutex},
  u8,
};
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::time::sleep;
use tokio_serial::SerialPortType::UsbPort;
use tower_http::services::ServeDir;

struct ApplicationState {
  users: Mutex<Vec<User>>,
  send: Sender<CarreraEvent>,
  messages: Mutex<Vec<String>>,
}

#[derive(Serialize, Copy, Clone, Debug)]
struct CarState {
  fuel_level: u8,
  in_pit: bool,
  speed: u8,
  last_lap: Option<SystemTime>,
}

impl CarState {
  fn default() -> Self {
    Self {
      fuel_level: 255,
      in_pit: false,
      last_lap: None,
      speed: 0,
    }
  }
}

#[derive(Serialize, Debug, Clone)]
enum CarreraEvent {
  ControllerUpdate(u8, u8),
  CarUpdate(u8, CarState),
  LightUpdate(u8),
  NewLap(u8, Duration),
  Reset,
}

fn open_arduino_port() -> Option<SerialStream> {
  for port in available_ports().unwrap_or_else(|_| Vec::new()) {
    match port.port_type {
      UsbPort(usb) => {
        match usb.manufacturer {
          Some(str) if str.contains("Arduino") => {
            return serialport::new(port.port_name.as_str(), 115200)
              .timeout(Duration::from_secs(20))
              .open_native_async()
              .ok();
          }
          _ => {}
        };
      }
      _ => {}
    }
  }
  None
}

fn handle_command(
  command: u8,
  data: u8,
  controller: u8,
  car_states: &mut [CarState; 8],
  bc_channel: &Sender<CarreraEvent>,
) {
  let now = SystemTime::now();
  let car: &mut CarState = car_states[controller as usize].borrow_mut();

  match command {
    16 => {
      //send.send("*".repeat(data as usize).to_string()).unwrap();
      bc_channel.send(CarreraEvent::LightUpdate(data)).unwrap();
    }
    4 => {
      if car.fuel_level != data {
        //send.send(format!("Fuel update [{}]: {}", controller, data)).unwrap();
        car.fuel_level = data;
        bc_channel
          .send(CarreraEvent::CarUpdate(controller, car.clone()))
          .unwrap();
      }
    }
    5 => {
      if data == 1 && !car.in_pit {
        //send.send(format!("Car [{}] enters the pit!", controller)).unwrap();
        car.in_pit = true;
        bc_channel
          .send(CarreraEvent::CarUpdate(controller, car.clone()))
          .unwrap();
      } else if data == 0 && car.in_pit {
        //send.send(format!("Car [{}] left the pit!", controller)).unwrap();
        car.in_pit = false;
        bc_channel
          .send(CarreraEvent::CarUpdate(controller, car.clone()))
          .unwrap();
      }
    }
    8..=9 => {
      let mut diff = Duration::from_secs(0);
      if let Some(prev) = car.last_lap {
        diff = now.duration_since(prev).unwrap_or(Duration::from_secs(0));
      }

      //send.send(format!("Car [{}] just completed a lap [{}]!", controller, diff.as_secs_f64())).unwrap();
      bc_channel
        .send(CarreraEvent::NewLap(controller, diff))
        .unwrap();
      car.last_lap = Some(now);
    }
    19 => {
      for state in car_states {
        state.last_lap = None;
      }

      bc_channel.send(CarreraEvent::Reset).unwrap();
    }
    _ => {}
  }
}

fn arduino_loop(send: Sender<CarreraEvent>) {
  tokio::spawn(async move {
    let mut arduino_opt = open_arduino_port();
    while arduino_opt.is_none() {
      println!("Waiting for arduino connection...");
      sleep(Duration::from_secs(10)).await;
      arduino_opt = open_arduino_port();
    }

    println!("Opened connection!");
    let arduino = arduino_opt.unwrap();
    let mut reader = BufReader::new(arduino);
    let mut buffer = String::new();

    let test: u8 = 8;

    let mut car_states = [CarState::default(); 8];

    loop {
      let _ = reader.read_line(&mut buffer).await.unwrap();
      // Remove \r\n from input string
      buffer.drain(buffer.len() - 2..);

      let tokens = buffer.split("-").collect::<Vec<&str>>();
      let program_data_word = tokens.get(0).unwrap();

      let pdw_items = program_data_word.split(";").collect::<Vec<&str>>();
      if pdw_items.len() >= 3 {
        let command: u8 = pdw_items.get(0).unwrap().parse().unwrap();
        let data: u8 = pdw_items.get(1).unwrap().parse().unwrap();
        let controller: u8 = pdw_items.get(2).unwrap().parse().unwrap();

        handle_command(command, data, controller, &mut car_states, &send);
      }

      if tokens.len() > 2 {
        let controller_word = tokens.get(1).unwrap();
        let cw_items = controller_word.split(";").collect::<Vec<&str>>();

        if cw_items.len() == 2 {
          let car_id: u8 = cw_items.get(0).unwrap().parse().unwrap();
          let speed: u8 = cw_items.get(1).unwrap().parse().unwrap();

          let car: &mut CarState = car_states[car_id as usize].borrow_mut();

          if car.speed != speed {
            car.speed = speed;
            send
              .send(CarreraEvent::ControllerUpdate(car_id, speed))
              .unwrap();
          }
        }
      }

      /*println!("Arduino says '{}'", &buffer);
      send.send(buffer.clone()).unwrap();*/
      buffer.clear();
    }
  });
}

#[tokio::main]
async fn main() {
  // initialize tracing
  tracing_subscriber::fmt::init();
  let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
  let static_files_service = ServeDir::new(assets_dir).append_index_html_on_directories(true);

  let (send, _recv) = broadcast::channel::<CarreraEvent>(16);
  arduino_loop(send.clone());

  let users: Mutex<Vec<User>> = Mutex::new(Vec::new());
  let messages: Mutex<Vec<String>> = Mutex::new(Vec::new());
  let app_state = Arc::new(ApplicationState {
    users,
    send,
    messages,
  });

  // build our application with a route
  let app = Router::new()
    .fallback_service(static_files_service)
    .route("/users", post(create_user))
    .with_state(app_state.clone())
    .route("/users", get(get_users))
    .with_state(app_state.clone())
    .route("/sse", get(sse_hander))
    .with_state(app_state.clone())
    .route("/messages", get(get_messages))
    .with_state(app_state);

  // run our app with hyper, listening globally on port 3000
  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  axum::serve(listener, app).await.unwrap();
}

async fn get_users(State(state): State<Arc<ApplicationState>>) -> (StatusCode, Json<Vec<User>>) {
  let users = state.users.lock().unwrap().clone();
  (StatusCode::OK, Json(users))
}

async fn get_messages(
  State(state): State<Arc<ApplicationState>>,
) -> (StatusCode, Json<Vec<String>>) {
  (StatusCode::OK, Json(state.messages.lock().unwrap().clone()))
}

async fn sse_hander(
  State(state): State<Arc<ApplicationState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
  let recv = state.send.subscribe();
  Sse::new(recv_to_stream(recv)).keep_alive(KeepAlive::default())
}

fn recv_to_stream(
  mut recv: Receiver<CarreraEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> {
  stream! {
    loop {
      match recv.recv().await {
        Ok(msg) => {
            let name: String;

            match msg {
              CarreraEvent::ControllerUpdate(_,_) => {
                name = String::from("Controller");
              },
              _ => {
                name = String::from("Arduino");
              }
            };

            //yield Ok(Event::default().event("Arduino").json_data(msg));
            match Event::default().event(name.as_str()).json_data(msg) {
              Ok(event) => {
                yield Ok(event);
              },
              Err(_err) => {
                yield Ok(Event::default().event(name.as_str()).data(""));
              }
            }
        }
        Err(_) => {
            break;
        }
      }
    }
  }
}

async fn create_user(
  State(state): State<Arc<ApplicationState>>,
  Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
  let mut users = state.users.lock().unwrap();

  let user = User {
    id: users.len() as u64,
    username: payload.username.clone(),
  };

  users.push(user.clone());
  (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
  username: String,
}

// the output to our `create_user` handler
#[derive(Serialize, Clone)]
struct User {
  id: u64,
  username: String,
}
