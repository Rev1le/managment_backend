#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]


use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::f64::NAN;
use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use tauri::WindowUrl::App;
use management_core::{SchemaError, CoefficientScheme,  Skill, Vacancy, VacancyCoefficient, Job, Company, JobLevel, Question, AnswerVariant};

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
    schema: CoefficientScheme
}

impl ManagementApp {
    pub fn new(config: &Path) -> Result<Self, AppError> {

        let config_f = File::open(config).unwrap();

        Ok(Self {
            schema: CoefficientScheme::new(config_f)?
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
    return percentage_correctness * 0.25 / all_percentage;

}

#[tauri::command]
fn get_questions(app: State<'_, Mutex<ManagementApp>>) -> HashSet<Question> {

    let schema = app.lock().unwrap();
    return schema.schema.get_questions().clone();

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnswerResponse {
    question_uuid: String,
    answers: Vec<String>
}

#[tauri::command]
fn get_questions_answers(app: State<'_, Mutex<ManagementApp>>, answers: Vec<QuestionAnswerResponse>) -> f64 {

    let schema = app.lock().unwrap();
    let questions_target = schema.schema.get_questions();

    let all_answer = answers.len();
    let mut correct_count = 0;

    for answer in answers {
        let question_target = questions_target.get(&answer.question_uuid);
        match question_target {
            None => continue,
            Some(question_target) => {

                let question_target_variants = question_target.get_variants();

                let result = answer.answers
                    .iter()
                    .fold(true, |res, answer| {
                        let answer_res = question_target_variants
                            .get(answer)
                            .map_or(false, |a| a.get_answer_state());

                        return match answer_res {
                            true => res,
                            false => false
                        }

                    });

                println!("{}", result);

                if result {
                    correct_count += 1;
                }
            }
        }

    }

    correct_count as f64 / all_answer as f64

}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnswerResultRequest {
    question_uuid: String,
    answer_result: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnswerResultResponse {
    question_title: String,
    answer_result: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponseSaveAnswers {
    answers: Vec<AnswerResultResponse>,
    name: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserSaveResult {
    name: String,
    test_results: Option<Vec<AnswerResultRequest>>,
    vacancy_results: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AllSave(pub Vec<UserSaveResult>);

#[tauri::command]
fn save_test(
    app: State<'_, Mutex<ManagementApp>>,
    tmp: State<'_, Mutex<Option<AllSave>>>,
    name: String,
    test_results: Option<Vec<AnswerResultRequest>>,
    vacancy_results: Option<f64>)
{
    println!("Имя студента {:?}", name);
    println!("Данные для сохранения теста {:?}", test_results);
    println!("Данные для сохранения навыков {:?}", vacancy_results);

    *tmp.lock().unwrap() = Some(AllSave(vec![UserSaveResult {
        name,
        test_results,
        vacancy_results,
    }]));

    return;

}

#[tauri::command]
fn get_saved_result(tmp: State<'_, Mutex<Option<AllSave>>>) -> AllSave {

    return tmp.lock().unwrap().clone().unwrap();
}

fn main() {

    let management_app = ManagementApp::new(Path::new("./skill_coefficients.json")).unwrap();

    tauri::Builder::default()
        .manage(Mutex::new(None::<AllSave>))
        .manage(Mutex::new(management_app))
        .invoke_handler(tauri::generate_handler![
            get_skills,
            get_vacancies,
            get_vacancies_for_worker,
            get_companies,
            check_placement,
            get_current_company,
            get_questions,
            get_questions_answers,
            save_test,
            get_saved_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    //tests::test();
}

mod tests {
    use std::path::Path;
    use std::sync::Mutex;
    use tauri::{Manager, State};
    use crate::{check_placement, get_companies, get_current_company, get_questions, get_questions_answers, get_vacancies_for_worker, ManagementApp, PlacementRequest, QuestionAnswerResponse, WorkerRequest};

    #[tauri::command]
    fn get_skills_ww(app: State<'_, Mutex<ManagementApp>>) -> i64 {
        return 2;
    }

    pub fn test() {

        let app = tauri::Builder::default()
            .setup(|app| {

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

                // println!(
                //     "Result get_jobs: {:?}",
                //     serde_json::to_string(&check_placement(man_app, placement, ))
                // );

                let questions = get_questions(man_app.clone());
                let mut question_iter = questions.iter();

                let mut answers = vec![];

                answers.push(QuestionAnswerResponse {
                    question_uuid: question_iter.next().unwrap().get_uuid().clone(),
                    answers: vec!["Да".into()],
                });
                answers.push(QuestionAnswerResponse {
                    question_uuid: question_iter.next().unwrap().get_uuid().clone(),
                    answers: vec!["Да".into()],
                });

                let res = get_questions_answers(man_app, answers);

                println!("{}", res);

                Ok(())

            })
            .invoke_handler(tauri::generate_handler![get_skills_ww])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
