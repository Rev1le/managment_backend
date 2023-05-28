use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ptr::Unique;
use std::vec::IntoIter;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeStruct;
use uuid::Uuid;

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
    pub name: String,
    pub vacancies_coefficient: Vec<VacancyCoefficient>
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
    pub name: String,
    pub tree: JobLevel
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

fn create_string_uuid() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    #[serde(default = "create_string_uuid")]
    uuid: String,
    title: String,
    variants: HashSet<AnswerVariant>
}

impl Question {

    pub fn get_uuid(&self) -> &String {
        &self.uuid
    }

    pub fn get_variants(&self) -> &HashSet<AnswerVariant> {
        &self.variants
    }

    pub fn get_title(&self) -> &String {
        &self.title
    }
}

impl Hash for Question {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state)
    }
}

impl PartialEq for Question {
    fn eq(&self, other: &Self) -> bool {
        self.title.eq(&other.title)
    }
}

impl Eq for Question {}

impl Borrow<String> for Question {
    fn borrow(&self) -> &String {
        &self.uuid
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerVariant {
    content: String,
    is_answer: bool
}

impl AnswerVariant {
    pub fn get_answer_state(&self) -> bool {
        self.is_answer
    }
}

impl Hash for AnswerVariant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content.hash(state)
    }
}

impl PartialEq for AnswerVariant {
    fn eq(&self, other: &Self) -> bool {
        self.content.eq(&other.content)
    }
}

impl Eq for AnswerVariant {}

impl Borrow<String> for AnswerVariant {
    fn borrow(&self) -> &String {
        &self.content
    }
}