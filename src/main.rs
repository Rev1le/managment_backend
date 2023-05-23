#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::WindowUrl::App;
use management_core::{SchemaError, CoefficientScheme, Team, Skill, Vacancy, VacancyCoefficient, Job};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    name: String,
    skills: Vec<String>
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkerResponse {
    pub name: String,
    pub vacancies: BTreeMap<String, i64>
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
fn get_jobs(app: State<'_, Mutex<ManagementApp>>) -> HashSet<Job> {

    let schema = app.lock().unwrap();
    let jobs = schema.schema.get_jobs().clone();

    return jobs;
}

#[tauri::command]
fn get_vacancies_for_worker(app: State<'_, Mutex<ManagementApp>>, worker: WorkerRequest) -> WorkerResponse {

    let schema = app.lock().unwrap();
    let mut vacancies_coefs: BTreeMap<String, i64> = BTreeMap::default();

    println!("{:#?}", schema.schema.get_skills());

    for skill in &worker.skills {
        println!("{}", skill);
        let skill_info = schema.schema
            .get_skills()
            .get(skill)
            .ok_or(AppError::Custom {
                name: "Not Found".into(),
                description: "Skill not found in schema".into()
            }).unwrap();

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

    let mut vac = vacancies_coefs.clone().into_iter().collect::<Vec<(String, i64)>>();
    vac.sort_by_key(|vac| vac.1);
    println!("Отсорт массив: {:?}", vac);

    //let schema = app.lock().unwrap();
    //let skills = schema.schema.get_vacancies().clone();

    return WorkerResponse {
        name: worker.name,
        vacancies: vacancies_coefs
    };
}

fn main() {
    // tauri::Builder::default()
    //     .manage(
    //         Mutex::new(ManagementApp::new(Path::new("./skill_coefficients.json")).unwrap())
    //     )
    //     .invoke_handler(tauri::generate_handler![get_skills, get_vacancies, get_vacancies_for_worker, get_jobs])
    //     .run(tauri::generate_context!())
    //     .expect("error while running tauri application");

    tests::test();
}

mod tests {
    use std::path::Path;
    use std::sync::Mutex;
    use tauri::{Manager, State};
    use crate::{get_jobs, get_vacancies_for_worker, ManagementApp, WorkerRequest};

    #[tauri::command]
    fn get_skills_ww(app: State<'_, Mutex<ManagementApp>>) -> i64 {
        return 2;
    }

    pub fn test() {

        let app = tauri::Builder::default()
            .setup(|app| {

                println!("GFWGWGGWG");

                app.manage(Mutex::new(ManagementApp::new(Path::new("./skill_coefficients.json")).unwrap()));

                let man_app = app.state::<Mutex<ManagementApp>>();

                let result = get_vacancies_for_worker(man_app.clone(), WorkerRequest {
                    name: "Олег".to_string(),
                    skills: vec!["Надёжность".into(), "Спокойствие".into()],
                });

                println!("Result: {:?}", result);

                println!("Result get_jobs: {:?}", get_jobs(man_app));

                Ok(())

            })
            .invoke_handler(tauri::generate_handler![get_skills_ww])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
