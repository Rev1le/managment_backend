#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;
use serde::Serialize;
use tauri::State;
use tauri::WindowUrl::App;
use management_core::{SchemaError, CoefficientScheme, Team, Skill, Vacancy, VacancyCoefficient};

#[derive(Debug)]
pub enum AppError {
    SchemaError(SchemaError),
    Custom {
        name: String,
        description: String
    }
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

#[derive(Debug, Clone, Serialize)]
pub struct WorkerRequest {
    name: String,
    skills: Vec<String>
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerResponse {
    name: String,
    vacancies: Vec<Vacancy>
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

#[tauri::command]
fn get_vacancies_for_worker(app: State<'_, Mutex<ManagementApp>>, worker: WorkerRequest) -> Result<BTreeMap<String, i64>, AppError> {

    let schema = app.lock().unwrap();
    let mut vacancies_coefs: BTreeMap<String, i64> = BTreeMap::default();

    for skill in &worker.skills {
        let skill_info = schema.schema
            .get_skills()
            .get(skill)
            .ok_or(AppError::Custom {
                name: "Not Found".into(),
                description: "Skill not found in schema".into()
            })?;

        let vacancies_coef = skill_info.get_vacancies_coefficient();

        for vac_coef in vacancies_coef {

            let vacancy_name = vac_coef.get_vacancy_name().clone();
            let coef = vac_coef.get_coefficient();

            vacancies_coefs
                .entry(vacancy_name)
                .and_modify(|e| {
                    println!("{}", e);
                    *e += coef;
                })
                .or_insert(coef);
        }
    }

    println!("{:?}", vacancies_coefs);

    //let schema = app.lock().unwrap();
    //let skills = schema.schema.get_vacancies().clone();

    return Ok(vacancies_coefs);
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
