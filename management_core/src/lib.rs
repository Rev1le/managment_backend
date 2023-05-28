#![feature(ptr_internals)]

mod models;

pub use models::*;

use std::{fs, io::{self, Read}, collections::{HashSet, HashMap}, hash::{Hash, Hasher}, rc::{Rc, Weak as RcWeak}, mem};
use std::arch::x86_64::CpuidResult;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Error;
use std::iter::Map;
use std::path::Path;
use std::ptr::Unique;
use std::slice::Iter;
use std::sync::{Arc, OnceLock, Weak as ArcWeak};
use std::vec::IntoIter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeStruct;
use uuid::Uuid;

use serde_json::{Value, from_reader};


#[derive(Debug)]
pub enum SchemaError {
    IoError(io::Error),
    Custom {
        name: String,
        description: String
    }
}

impl From<io::Error> for SchemaError {
    fn from(value: Error) -> Self {
        Self::IoError(value)
    }
}

// Связать vacancies и skills обычной ссылкой с верменем жизни, а не RC
#[derive(Debug, Clone, Serialize)]
pub struct CoefficientScheme {
    vacancies: HashSet<Vacancy>,
    skills: HashSet<Skill>,
    companies: HashSet<Company>,
    questions: HashSet<Question>,
}

impl CoefficientScheme {
    pub fn new(mut schema_f: fs::File) -> Result<Self, SchemaError> {

        let mut schema_bytes = vec![];
        schema_f.read_to_end(&mut schema_bytes)?;

        let json: Value = serde_json::from_slice(&schema_bytes).unwrap();
        println!("------------\nJson schema: {:#?}\n------------", json);

        let vacancies =
            CoefficientScheme::parse_vacancies(&json["vacancies"]);
        println!("Успешных парсинг вакансий\n------------");

        let skills =
            CoefficientScheme::parse_skills(&json["skills"], &vacancies);
        println!("Успешных парсинг навыков\n------------");

        let companies =
            CoefficientScheme::parse_companies(&json["jobs"]["companies"]);
        println!("Успешных парсинг компаний\n------------");

        let questions =
            CoefficientScheme::parse_questions(&json["questions"]);
        println!("Успешных парсинг вопросов------------");

        return Ok(Self {
            vacancies,
            skills,
            companies,
            questions
        });
    }

    fn parse_vacancies(value: &Value) -> HashSet<Vacancy> {
        let iter = value
            .as_array()
            .expect("json конфиг не содержит массива в поле 'vacancies'")
            .iter()
            .map(
                |vacancy| {
                    let vacancy_name_str = vacancy
                        .as_str()
                        .unwrap();
                    return Vacancy(vacancy_name_str.into());
                }
            );

        return HashSet::from_iter(iter);
    }

    fn parse_skills(value: &Value, vacancies: &HashSet<Vacancy>) -> HashSet<Skill> {

        let skills = value
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'skills'");

        let mut res_skills = HashSet::default();

        for (skill_name, vacancies_coef) in skills {

            let vacancies_coef_map = vacancies_coef.as_object().unwrap();
            let mut vacancies_coefficient = Vec::default();

            for (vacancy_name, vacancy_coef) in vacancies_coef_map {

                let vacancy: Vacancy = vacancy_name.clone().into();
                let vacancy_rc = vacancies
                    .get::<Vacancy>(&vacancy)
                    .ok_or(
                        SchemaError::Custom {
                            name: "Not found vacancy in HasSet".to_owned(),
                            description: format!("В хешсете с вакансиями не найдена вакансия: {:?}", vacancy_name)
                        }
                    ).unwrap();

                let vacancy_coefficient = VacancyCoefficient::new(
                    &*vacancy_rc,
                    vacancy_coef.as_i64().unwrap()
                );

                vacancies_coefficient.push(vacancy_coefficient);
            }

            res_skills.insert( Skill {
                name: skill_name.clone(),
                vacancies_coefficient
            });
        }

        return res_skills;
    }

    // fn parse_jobs(value: &Value) -> HashSet<Job> {
    //
    //     let jobs_map = value
    //         .as_object()
    //         .expect("json конфиг не содержит объекта в поле 'jobs'")
    //         ["companies"]
    //         .as_object()
    //         .expect("json конфиг не содержит объекта в поле 'companies'");
    //
    //     return jobs_map
    //         .into_iter()
    //         .map(|job| {
    //             //println!("{}", job.1);
    //             //let job_graph = dbg!(serde_json::from_value::<JobLevel>(job.1.clone()).unwrap());
    //             Job(job.0.into())
    //         })
    //         .collect::<HashSet<Job>>();
    // }

    fn parse_companies(value: &Value) -> HashSet<Company> {

        let companies_map = value
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'companies'");

        return companies_map
            .into_iter()
            .map(|company| {
                //println!("{}", job.1);
                let company_graph =
                    serde_json::from_value::<JobLevel>(company.1.clone())
                        .unwrap();
                Company {
                    name: company.0.into(),
                    tree: company_graph
                }
            })
            .collect::<HashSet<Company>>();
    }

    fn parse_questions(value: &Value) -> HashSet<Question> {
        serde_json::from_value::<HashSet<Question>>(value.clone())
            .expect("Пасинг вопрос произошел неудачно")
    }

    // Not use pls
    pub fn delete_all_vacancies(&mut self) {
        self.vacancies.remove(&Vacancy("глава".into()));
        self.vacancies.insert(Vacancy("Hackme aaaa".into()));

        println!("{:?}", self.vacancies);
    }

    pub fn get_vacancies(&self) -> &HashSet<Vacancy> {
        &self.vacancies
    }

    pub fn get_skills(&self) -> &HashSet<Skill> {
        &self.skills
    }

    // pub fn get_jobs(&self) -> &HashSet<Job> {
    //     &self.jobs
    // }

    pub fn get_companies(&self) -> &HashSet<Company> {
        &self.companies
    }

    pub fn get_questions(&self) -> &HashSet<Question> {
        &self.questions
    }

    pub fn get_question_by_uuid(&self, uuid: &String) -> Question {
        let q = self.questions.get(&uuid.clone());

        q.unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use super::*;

    #[test]
    fn it_works() {

        let f = File::open("../skill_coefficients.json").unwrap();

        let mut schema = CoefficientScheme::new(f).unwrap();
        println!("\n{:#?}", schema);
        //println!("\n{:?}", schema.skills.iter().next().unwrap().vacancies_coefficient[0].0.upgrade());
        assert_eq!(4, 4);
    }
}
