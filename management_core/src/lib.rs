#![feature(ptr_internals)]

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
    //jobs: HashSet<Job>,
    companies: HashSet<Company>,
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

        let companies =
            CoefficientScheme::parse_companies(&json["jobs"]["companies"]);
        println!("Успешных парсинг компаний\n------------");

        return Ok(Self {
            vacancies,
            skills,
            //jobs,
            companies
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

    fn parse_jobs(value: &Value) -> HashSet<Job> {

        let jobs_map = value
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'jobs'")
            ["companies"]
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'companies'");

        return jobs_map
            .into_iter()
            .map(|job| {
                //println!("{}", job.1);
                //let job_graph = dbg!(serde_json::from_value::<JobLevel>(job.1.clone()).unwrap());
                Job(job.0.into())
            })
            .collect::<HashSet<Job>>();
    }

    fn parse_companies(value: &Value) -> HashSet<Company> {

        let companies_map = value
            .as_object()
            .expect("json конфиг не содержит объекта в поле 'companies'");

        return companies_map
            .into_iter()
            .map(|company| {
                //println!("{}", job.1);
                let company_graph = dbg!(
                    serde_json::from_value::<JobLevel>(company.1.clone())
                        .unwrap()
                );
                Company {
                    name: company.0.into(),
                    tree: company_graph
                }
            })
            .collect::<HashSet<Company>>();
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

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Job(String);

// impl Hash for Job {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.0.hash(state)
//     }
// }
//
// impl PartialEq for Job {
//     fn eq(&self, other: &Self) -> bool {
//         self.0.eq(&other.0)
//     }
// }
//
// impl Eq for Job {}


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

#[derive(Debug, Clone, Deserialize)]
pub struct JobLevel {
    label: HashMap<String, String>,
    children: Option<Vec<JobLevel>>,
}

impl JobLevel {
    pub fn label(&self) -> &HashMap<String, String> {
        &self.label
    }

    pub fn get_iter(&self) -> IntoIter<JobLevel> {
        let mut v_all_children = vec![];
        v_all_children.push(self.clone());

        if let Some(levels) = &self.children {
            for level in levels {
                let tmp_levels = level.get_iter();
                v_all_children.extend(tmp_levels);
            }
        }

        return v_all_children.into_iter()
    }

}


impl Serialize for JobLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {

        let mut s = serializer
            .serialize_struct("JobLevel", 2)?;

        s.serialize_field("label", &self.label
            .iter()
            .next()
            .expect("JobLevel не содержит label")
            .0
        )?;

        if let Some(children) = &self.children {
            s.serialize_field("children", children)?;
        }

        s.end()
    }
}

impl IntoIterator for JobLevel {
    type Item = JobLevel;
    type IntoIter = IntoIter<JobLevel>;

    fn into_iter(self) -> Self::IntoIter {
        let self_cl = self.clone();

        if let Some(levels) = self.children {

            let children_iter = levels.into_iter();
            //levels.push(self_cl);
            return [self_cl.into_iter(), children_iter].into_iter().flatten().collect::<Vec<JobLevel>>().into_iter()
        }

        Vec::new().into_iter()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    name: String,
    tree: JobLevel
}

impl Company {
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn tree(&self) -> &JobLevel {
        &self.tree
    }
}

impl Hash for Company {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl PartialEq for Company {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq(&other.name)
    }
}

impl Eq for Company {}

impl Borrow<String> for Company {
    fn borrow(&self) -> &String {
        &self.name
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
