#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;
use serde::Serialize;
use tauri::State;
use management_core::{SchemaError, CoefficientScheme, Team, Skill, Vacancy};

#[derive(Debug)]
pub enum AppError {
    SchemaError(SchemaError)
}

impl From<SchemaError> for AppError {
    fn from(value: SchemaError) -> Self {
        AppError::SchemaError(value)
    }
}

#[derive(Serialize)]
pub struct ManagementApp {
    schema: CoefficientScheme,
    team: Mutex<Team>
}

impl ManagementApp {
    pub fn new(config: &Path) -> Result<Self, AppError> {

        let config_f = File::open(config).unwrap();

        Ok(Self {
            schema: CoefficientScheme::new(config_f)?,
            team: Mutex::new(Team::new()),
        })
    }
}


#[tauri::command]
fn get_skills(app: State<'_, Mutex<ManagementApp>>) -> HashSet<Skill> {

    let schema = app.lock().unwrap();
    let skills = schema.schema.get_skills().clone();

    return skills;
}

#[tauri::command]
fn get_vacancies(app: State<'_, Mutex<ManagementApp>>) -> HashSet<Vacancy> {

    let schema = app.lock().unwrap();
    let skills = schema.schema.get_vacancies().clone();

    return skills;
}

fn main() {
    tauri::Builder::default()
        .manage(
            Mutex::new(ManagementApp::new(Path::new("./skill_coefficients.json")).unwrap())
        )
        .invoke_handler(tauri::generate_handler![get_skills, get_vacancies])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
