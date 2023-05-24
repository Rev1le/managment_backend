#![feature(ptr_internals)]

use std::{fs, io::{self, Read}, collections::{HashSet, HashMap}, hash::{Hash, Hasher}, rc::{Rc, Weak as RcWeak}, mem};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Error;
use std::path::Path;
use std::ptr::Unique;
use std::sync::{Arc, OnceLock, Weak as ArcWeak};
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeStruct;

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

#[derive(Debug, Clone, Serialize)]
pub struct Team(Vec<Worker>);

impl Team {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add_worker(&mut self, worker: Worker) {
        self.0.push(worker);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Worker {
    name: String,
    skills: Vec<Skill>
}

impl Worker {
    pub fn new(name: &str, skills: &[Skill]) -> Self {
        Self {
            name: name.into(),
            skills: skills.into()
        }
    }

    pub fn get_suitable_vacancy(&self) -> Vacancy {
        todo!()
    }
}

// Связать vacancies и skills обычной ссылкой с верменем жизни, а не RC
#[derive(Debug, Clone, Serialize)]
pub struct CoefficientScheme {
    vacancies: HashSet<Vacancy>,
    skills: HashSet<Skill>,
    jobs: HashSet<Job>,
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

        let jobs =
            CoefficientScheme::parse_jobs(&json["jobs"]);
        println!("Успешных парсинг работ\n------------");

        return Ok(Self { vacancies, skills, jobs });
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

    fn parse_jobs(value: &Value) -> HashSet<Job> {

        let jobs_map = value
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'jobs'")
            ["companies"]
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'companies'");

        return jobs_map
            .into_iter()
            .map(|job| Job(job.0.into()))
            .collect::<HashSet<Job>>();
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

    pub fn get_jobs(&self) -> &HashSet<Job> {
        &self.jobs
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Vacancy(pub String);

impl Hash for Vacancy {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl From<String> for Vacancy {
    fn from(value: String) -> Self {
        Vacancy(value)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    name: String,
    vacancies_coefficient: Vec<VacancyCoefficient>
}

impl Skill {
    pub fn get_vacancies_coefficient(&self) -> &Vec<VacancyCoefficient> {
        &self.vacancies_coefficient
    }
}

impl Borrow<String> for Skill {
    fn borrow(&self) -> &String {
        &self.name
    }
}

impl Hash for Skill {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl PartialEq for Skill {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Skill {}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Job(String);

#[derive(Clone)]
pub struct VacancyCoefficient(Unique<Vacancy>, i64);

impl VacancyCoefficient {
    pub fn new(vacancy: &Vacancy, coefficient: i64) -> Self {

        // Безопасность вышла покурить
        let v = vacancy as *const Vacancy;
        let vacancy_ptr = v as *mut Vacancy;

        Self(Unique::new(vacancy_ptr).unwrap(), coefficient)
    }

    pub fn get_vacancy_name(&self) -> String {
        let vacancy_name = unsafe {
            self.0.as_ref().0.clone()
        };

        return vacancy_name
    }

    pub fn get_coefficient(&self) -> i64 {
        self.1
    }
}

impl Debug for VacancyCoefficient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        let vacancy_ref = unsafe {
            self.0.as_ref()
        };

        write!(f, "VacancyCoefficient({:?}, {})", vacancy_ref, self.1)
    }
}

impl Serialize for VacancyCoefficient {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let vacancy = unsafe{
            self.0.as_ref()
        };

        let mut s = serializer.serialize_struct("VacancyCoefficient", 2)?;
        s.serialize_field("vacancy", &vacancy)?;
        s.serialize_field("coefficient", &self.1)?;
        s.end()
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
