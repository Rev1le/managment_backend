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
use serde_json::Value;
use tauri::State;
use tauri::WindowUrl::App;
use management_core::{SchemaError, CoefficientScheme, Team, Skill, Vacancy, VacancyCoefficient, Job, Company, JobLevel};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementRequest {
    company_name: String,
    placements: HashMap<String, Value>
}

#[tauri::command]
fn get_skills(app: State<'_, Mutex<ManagementApp>>) -> HashSet<Skill> {

    let schema = app.lock().unwrap();
    let skills = schema.schema.get_skills().clone();
    println!("Возвращены навыки\n------------");

    return skills;
}

#[tauri::command]
fn get_vacancies(app: State<'_, Mutex<ManagementApp>>) -> HashSet<Vacancy> {

    let schema = app.lock().unwrap();
    let vacancies = schema.schema.get_vacancies().clone();
    println!("Возвращены должности\n------------");

    return vacancies;
}

#[tauri::command]
fn check_placement(app: State<'_, Mutex<ManagementApp>>, data: PlacementRequest) -> f64 {

    let schema = &app.lock().unwrap().schema;
    let current_company_opt = schema
        .get_companies()
        .get(&data.company_name);

    let company;

    if let Some(current_company) = current_company_opt {
        company = current_company.clone();
    } else {
        return 0.0
    }
    println!("{:?}", company);

    let vacancies_tree = company.tree();
    let vacancies_vec = vacancies_tree
        .get_iter()
        .collect::<Vec<JobLevel>>();

    let mut percentage_correctness: f64 = 0.0;
    let mut all_percentage: f64 = 0.0;

    for job_level in vacancies_vec {

        let (company_vacancy_name, target_vacancy_name) = job_level.label().iter().next().unwrap();
        let worker_data = data.placements.get(company_vacancy_name).unwrap_or(&Value::Null);

        if *worker_data == Value::Null {
            println!("Должность {} не проставлена на графе и не найдена в RequestData", company_vacancy_name);
            continue
        }

        let worker_vacancies_top = worker_data["vacancies"].as_array().unwrap();

        worker_vacancies_top.iter().enumerate().map(|(ind, vacancy)| {

            println!("{} ?==? {}\n", vacancy, target_vacancy_name);
            let vacancy = vacancy.as_str().unwrap();
            if vacancy == target_vacancy_name {
                percentage_correctness += 1.0/(1.0+ind as f64);
                println!("{}", percentage_correctness);
            }
        }).for_each(drop);

        all_percentage += 1.0;

        println!("Company vacancy name: {}, target vacancy name: {}. Worker top vacancies: {:?}", company_vacancy_name, target_vacancy_name, worker_vacancies_top);

    }

    println!("{} / {}", percentage_correctness, all_percentage);

    return percentage_correctness / all_percentage;


/*

        if let Some(current_company) = current_company_opt {
        let vacancies_tree = current_company.tree();

        let mut percentage_correctness = 0;
        let mut all_percentage = 0;

        for target_vacancy in vacancies_tree.get_iter() {

            let (target_vacancy_name, company_vacancy_name) = target_vacancy.label().iter().next().unwrap();

            let request_vacancy_company_data = data.placements
                .get(company_vacancy_name).unwrap_or(&Value::Null);

            if request_vacancy_company_data == Value::Null {
                println!("Должность {} не найдена в RequestData", company_vacancy_name);
                continue
            }

            let (request_vacancy_company_name, worker_data) = request_vacancy_company_data
                .as_object()
                .unwrap()
                .iter()
                .next()
                .expect(&format!(
                    "Должность {} не явялется валидной. Debug: {:?}",
                    company_vacancy_name,
                    request_vacancy_data)
                );
            let worker_vacancies_top = worker_data["vacancies"].as_array().unwrap();

            let vacancy_name = target_vacancy.label().keys().next().unwrap();
            let request_data_vacancies = data.placements.get(vacancy_name).unwrap().as_object().unwrap();

            let request_vacancies = request_data_vacancies["vacancies"].as_array().unwrap();

            for i in 0..=2 {
                let best_vacancy = &request_vacancies[i];

                if best_vacancy == label.label().values().next().unwrap() {
                    percentage_correctness += 1/(1+i);
                    break
                }
            }

            all_percentage += 1;

            // let best_vacancy = request_data_vacancies["vacancies"].as_array().unwrap()[0].as_str().unwrap();
            //
            // if best_vacancy == label.label().keys().next().unwrap() {
            //     percentage_correctness += 1;
            // }
            // all_percentage += 1;

            println!("{:?} -- {:#?}", label.label(), data);
        }

        println!("{} {}", percentage_correctness, all_percentage);

        if percentage_correctness / all_percentage >= 0 {
            return true
        }
    }

    return false

 */
}

// #[tauri::command]
// fn get_jobs(app: State<'_, Mutex<ManagementApp>>) -> HashSet<Job> {
//
//     let schema = app.lock().unwrap();
//     let jobs = schema.schema.get_jobs().clone();
//
//     return jobs;
// }

#[tauri::command]
fn get_companies(app: State<'_, Mutex<ManagementApp>>) -> Vec<String> {

    let schema = app.lock().unwrap();
    let companies = schema.schema.get_companies().iter().map(|company| company.name()).cloned().collect::<Vec<String>>();

    println!("Возвращены компании\n------------");
    return companies;
}

#[tauri::command]
fn get_current_company(app: State<'_, Mutex<ManagementApp>>, company_name: String) -> Option<Company> {

    let schema = app.lock().unwrap();
    let opt_company = schema.schema
        .get_companies()
        .get(&company_name);

    if let Some(company) = opt_company {
        println!("Возвращена компании {}\n------------", company.name());
        return Some(company.clone());
    }

    return None;
}

#[tauri::command]
fn get_vacancies_for_worker(
    app: State<'_, Mutex<ManagementApp>>,
    worker: WorkerRequest)
    -> WorkerResponse {

    let schema = app.lock().unwrap();
    let schema_skills = schema.schema.get_skills();

    let mut vacancies_coefs: BTreeMap<String, i64> = BTreeMap::default();

    for skill in &worker.skills {

        let skill_info = schema_skills
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
                    *e += coef;
                })
                .or_insert(coef);
        }
    }

    // Сортировка коеффициентов у должностей
    /*
    let mut vac = vacancies_coefs
        .clone()
        .into_iter()
        .collect::<Vec<(String, i64)>>();
    vac.sort_by_key(|vac| vac.1);

     */

    println!("Возвращены должности для работника: {}\n------------", worker.name);

    return WorkerResponse {
        name: worker.name,
        vacancies: vacancies_coefs
    };
}

fn main() {

    let management_app = ManagementApp::new(Path::new("./skill_coefficients.json")).unwrap();

    tauri::Builder::default()
        .manage(Mutex::new(management_app))
        .invoke_handler(tauri::generate_handler![
            get_skills,
            get_vacancies,
            get_vacancies_for_worker,
            get_companies,
            check_placement,
            get_current_company
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    //tests::test();
}

mod tests {
    use std::path::Path;
    use std::sync::Mutex;
    use tauri::{Manager, State};
    use crate::{check_placement, get_companies, get_current_company, get_vacancies_for_worker, ManagementApp, PlacementRequest, WorkerRequest};

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

                // let result = get_vacancies_for_worker(man_app.clone(), WorkerRequest {
                //     name: "Олег".to_string(),
                //     skills: vec!["Надёжность".into(), "Спокойствие".into()],
                // });
                //
                // println!("Result: {:?}", result);

                //println!("Result get_jobs: {:?}", serde_json::to_string(&get_current_company(man_app, "Разработка_ПО".into())));
                //println!("Result get_jobs: {:?}", serde_json::to_string(&get_companies(man_app.clone())));
                // println!(
                //     "Result get_jobs: {:?}",
                //     serde_json::to_string(
                //         &get_current_company(
                //             man_app,
                //             "Системная_интеграция".into()
                //         )
                //     )
                // );

                let json = r#"{"company_name":"Консалтинг","placements":{"Ьизнес-аналитик":{"name":"jrwj","qualities":["Пунктуальность","Спокойствие"],"id":4,"vacancies":["Analytic","Technical_Support","Manager","Programmer","QA_Engineer","Team_Lead"]},"Управляющий партнёр":{"name":"j","qualities":["Исполнительность","Ответственность"],"id":1,"vacancies":["Analytic","Manager","Programmer","Team_Lead","QA_Engineer"]},"Ведущий программист":{"name":"t","qualities":["Внимательность","Исполнительность"],"id":0,"vacancies":["System_Admin","Analytic","Programmer","QA_Engineer","Team_Lead"]},"Консультант":{"name":"jrwj","qualities":["Пунктуальность","Спокойствие"],"id":4,"vacancies":["HR_Manager","Technical_Support","Manager","Programmer","QA_Engineer","Team_Lead"]}}}"#;
                let placement = serde_json::from_str::<PlacementRequest>(json).unwrap();

                println!(
                    "Result get_jobs: {:?}",
                    serde_json::to_string(&check_placement(man_app, placement, ))
                );
                Ok(())

            })
            .invoke_handler(tauri::generate_handler![get_skills_ww])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
