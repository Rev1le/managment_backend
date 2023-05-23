#![feature(ptr_internals)]

use std::{fs, io::{self, Read}, collections::{HashSet, HashMap}, hash::{Hash, Hasher}, rc::{Rc, Weak as RcWeak}, mem};
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Error;
use std::path::Path;
use std::ptr::Unique;
use std::sync::{Arc, OnceLock, Weak as ArcWeak};
use serde::{Serialize, Serializer};
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
}

impl CoefficientScheme {
    pub fn new(mut schema_f: fs::File) -> Result<Self, SchemaError> {

        let mut schema_bytes = vec![];
        schema_f.read_to_end(&mut schema_bytes)?;

        let json: Value = serde_json::from_slice(&schema_bytes).unwrap();
        //println!("{:?}", json);

        let iter = json["vacancies"]
            .as_array()
            .expect("json конфиг не содержит массива в поле 'vacancies'")
            .iter()
            .map(
                |vacancy| {
                    let vacancy_name_str = vacancy.as_str().unwrap().to_owned();
                    return Vacancy(vacancy_name_str);
                }
            );

        let vacancies: HashSet<Vacancy> = HashSet::from_iter(iter);

        let mut res_skills: HashSet<Skill> = HashSet::default();
        let skills = json["skills"]
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'skills'");

        let mut schema = Self {
            vacancies,
            skills: HashSet::default(),
        };

        for (skill_name, vacancies_coef) in skills {

            let vacancies_coef_map = vacancies_coef.as_object().unwrap();
            let mut vacancies_coefficient = Vec::default();

            for (vacancy_name, vacancy_coef) in vacancies_coef_map {

                println!("{:?}", vacancy_coef);
                let vacancy: Vacancy = vacancy_name.clone().into();

                let vacancy_rc = schema.vacancies
                    .get::<Vacancy>(&vacancy)
                    .ok_or(
                        SchemaError::Custom {
                            name: "Not found vacancy in HasSet".to_owned(),
                            description: format!("В хешсете с вакансиями не найдена вакансия: {:?}", vacancy_name)
                        }
                    )?;

                //println!("{:?}", vacancy_rc);
                //println!("{:?}", vacancy_coef);

                vacancies_coefficient.push(VacancyCoefficient::new(&*vacancy_rc, vacancy_coef.as_i64().unwrap()));
                //vacancies_coefficient.push(VacancyCoefficient(Rc::downgrade(vacancy_rc), vacancy_coef.as_i64().unwrap()));
            }

            schema.skills.insert( Skill {
                name: skill_name.clone(),
                vacancies_coefficient
            });
        }

        Ok(schema)
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

#[derive(Clone)]
pub struct VacancyCoefficient(Unique<Vacancy>, i64);

impl VacancyCoefficient {
    pub fn new(vacancy: &Vacancy, coefficient: i64) -> Self {

        // Безопасность вышла покурить
        let v = vacancy as *const Vacancy;
        let vacancy_ptr = v as *mut Vacancy;

        Self(Unique::new(vacancy_ptr).unwrap(), coefficient)
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
