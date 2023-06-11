use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GameState {
    id: u64,
    word: String,
    guessed_letters: Vec<char>,
    incorrect_attempts: u8,
    last_move: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    games: HashMap<u64, GameState>,
}

impl Database {
    fn new() -> Self {
        Self {
            games: HashMap::new(),
        }
    }

    fn insert(&mut self, game: GameState) {
        self.games.insert(game.id, game);
    }

    fn get(&self, id: &u64) -> Option<&GameState> {
        self.games.get(id)
    }

    fn update(&mut self, game: GameState) {
        self.games.insert(game.id, game);
    }

    fn save_to_file(&self) -> std::io::Result<()> {
        let data: String = serde_json::to_string(&self)?;
        let mut file: fs::File = fs::File::create("database.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> std::io::Result<Self> {
        let file_content: String = fs::read_to_string("database.json")?;
        let db: Database = serde_json::from_str(&file_content)?;
        Ok(db)
    }
}

struct AppState {
    db: Mutex<Database>,
}

async fn start_game(app_state: web::Data<AppState>, word: web::Json<String>) -> impl Responder {
    let mut db: std::sync::MutexGuard<Database> = app_state.db.lock().unwrap();
    let game = GameState {
        id: rand::random(),
        word: word.into_inner(),
        guessed_letters: Vec::new(),
        incorrect_attempts: 0,
        last_move: String::new(),
    };
    db.insert(game.clone());
    let _ = db.save_to_file();
    HttpResponse::Ok().json(game)
}

async fn make_move(app_state: web::Data<AppState>, id: web::Path<u64>, letter: web::Json<char>) -> impl Responder {
    let mut db: std::sync::MutexGuard<Database> = app_state.db.lock().unwrap();
    let mut game = match db.get(&id.into_inner()).cloned() {
        Some(game) => game,
        None => return HttpResponse::NotFound().finish(),
    };

    let letter_inner = letter.into_inner();
    game.last_move = format!("Guessed letter: {}", letter_inner);
    game.guessed_letters.push(letter_inner);

    if !game.word.contains(letter_inner) {
        game.incorrect_attempts += 1;
    }

    db.update(game.clone());
    let _ = db.save_to_file();
    HttpResponse::Ok().json(game)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db: Database = match Database::load_from_file() {
        Ok(db) => db,
        Err(_) => Database::new(),
    };

    let data: web::Data<AppState> = web::Data::new(AppState {
        db: Mutex::new(db),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600),
            )
            .app_data(data.clone())
            .route("/start", web::post().to(start_game))
            .route("/move/{id}", web::post().to(make_move))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}