use candid::CandidType;
use ic_cdk::api::{id, time};
use ic_cdk::{init,query,update};
//use ic_cdk_macros::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Clone, Debug, Serialize, Deserialize,CandidType)]
pub struct Content {
    pub id: u64,
    pub content: String,
    pub author: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, CandidType)]
pub struct Report {
    pub content_id: u64,
    pub reason: String,
    pub reporter: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Vote {
    Remove,
    Keep,
}

#[derive(Clone, Debug, Serialize, Deserialize, CandidType)]
pub struct CommunityGuideline {
    pub rule: String,
    pub votes: u64,
    pub weighted_votes: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub reputation: u64,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        contents: Vec::new(),
        reports: Vec::new(),
        votes: Vec::new(),
        community_guidelines: Vec::new(),
        users: Vec::new(),
    });
}

pub struct State {
    pub contents: Vec<Content>,
    pub reports: Vec<Report>,
    pub votes: Vec<(u64, Vote)>,
    pub community_guidelines: Vec<CommunityGuideline>,
    pub users: Vec<User>,
}

#[init]
fn init() {
    // Initialization is now handled by the thread_local! macro
}

#[update]
fn submit_content(mut content: Content) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        //content.id = state.contents.len() as u64;
        state.contents.push(content);
    });
    Ok(())
}

#[update]
fn report_content(content_id: u64, reason: String) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let reporter = id().to_string();
        let report = Report { content_id, reason, reporter, timestamp: time() };
        state.reports.push(report);
    });
    Ok(())
}

fn vote(content_id: u64, vote: Vote) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let voter = id().to_string();

        // Check if content is removed before modifying user reputation
        let content_removed = content_is_removed(&state, content_id);

        // Find the user who cast the vote and update reputation
        if let Some(user) = state.users.iter_mut().find(|u| u.id == voter) {
            if content_removed {
                user.reputation += 10; // Increase reputation for correct vote
            } else {
                user.reputation = user.reputation.saturating_sub(5); // Decrease reputation for incorrect vote
            }
        }

        state.votes.push((content_id, vote));
    });
    Ok(())
}

#[query]
fn get_content(content_id: u64) -> Option<Content> {
    STATE.with(|state| {
        state.borrow().contents.iter().find(|c| c.id == content_id).cloned()
    })
}

#[query]
fn get_reports(content_id: u64) -> Vec<Report> {
    STATE.with(|state| {
        state.borrow().reports.iter().filter(|r| r.content_id == content_id).cloned().collect()
    })
}

#[update]
fn propose_guideline(rule: String) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.community_guidelines.push(CommunityGuideline { rule, votes: 0, weighted_votes: 0.0 });
    });
    Ok(())
}

#[update]
fn vote_guideline(rule_id: usize, vote: bool) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let voter = id().to_string();

        // Find the user who cast the vote
        let weight = state.users.iter().find(|u| u.id == voter)
            .map(|user| user.reputation as f32)
            .unwrap_or(1.0); // Default weight for unknown users

        // Update the guideline's vote count and weighted vote sum
        if let Some(guideline) = state.community_guidelines.get_mut(rule_id) {
            guideline.votes += 1;
            guideline.weighted_votes += if vote { weight } else { -weight };
        }
    });
    Ok(())
}

#[query]
fn get_guidelines() -> Vec<CommunityGuideline> {
    STATE.with(|state| state.borrow().community_guidelines.clone())
}

#[update]
fn update_reputation(user_id: String, delta: i64) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(user) = state.users.iter_mut().find(|u| u.id == user_id) {
            user.reputation = (user.reputation as i64 + delta).max(0) as u64;
        }
    });
    Ok(())
}

#[update]
fn moderate_content(content_id: u64) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let votes = state.votes.iter().filter(|v| v.0 == content_id).count();
        if votes > 5 { // Adjust threshold as needed
            if let Some(content) = state.contents.iter().find(|c| c.id == content_id) {
                for guideline in &state.community_guidelines {
                    if content.content.contains(&guideline.rule) {
                        // Remove content
                        state.contents.retain(|c| c.id != content_id);
                        // Update user reputations
                        let reporters: Vec<String> = state.reports.iter()
                            .filter(|r| r.content_id == content_id)
                            .map(|r| r.reporter.clone())
                            .collect();
                        for reporter in reporters {
                            update_reputation(reporter, 10)?; // Adjust reward
                        }
                        break;
                    }
                }
            }
        }
        Ok(())
    })
}

fn content_is_removed(state: &State, content_id: u64) -> bool {
    !state.contents.iter().any(|c| c.id == content_id)
}

#[query]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
