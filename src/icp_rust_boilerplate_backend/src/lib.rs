use ic_cdk::api::{call, id};
use ic_cdk_macros::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Content {
    pub id: u64,
    pub content: String,
    pub author: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommunityGuideline {
    pub rule: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub reputation: u64,
}

#[state]
pub struct State {
    pub contents: Vec<Content>,
    pub reports: Vec<Report>,
    pub votes: Vec<(u64, Vote)>,
    pub community_guidelines: Vec<CommunityGuideline>,
    pub users: Vec<User>,
}

#[init]
fn init() -> Result<(), String> {
    Ok(())
}

#[query]
fn submit_content(content: Content) -> Result<(), String> {
    let mut state = State::get();
    content.id = state.contents.len() as u64;
    state.contents.push(content.clone());
    state.set(state);
    Ok(())
}

#[query]
fn report_content(content_id: u64, reason: String) -> Result<(), String> {
    let mut state = State::get();
    let reporter = id().to_string();
    let report = Report { content_id, reason, reporter, timestamp: ic_cdk::api::time() };
    state.reports.push(report);
    state.set(state);
    Ok(())
}

#[query]
fn vote(content_id: u64, vote: Vote) -> Result<(), String> {
    let mut state = State::get();
    let voter = id().to_string();

    // Find the user who cast the vote
    let user = state.users.iter_mut().find(|u| u.id == voter);

    // Update the user's reputation based on the vote and content removal
    if let Some(user) = user {
        if content_is_removed(content_id) {
            user.reputation += 10; // Increase reputation for correct vote
        } else {
            user.reputation -= 5; // Decrease reputation for incorrect vote
        }
    }

    state.votes.push((content_id, vote));
    state.set(state);
    Ok(())
}

#[query]
fn get_content(content_id: u64) -> Option<Content> {
    State::get().contents.iter().find(|c| c.id == content_id).cloned()
}

#[query]
fn get_reports(content_id: u64) -> Vec<Report> {
    State::get().reports.iter().filter(|r| r.content_id == content_id).cloned().collect()
}

// ... other functions for community guidelines, reputation system, etc.

#[query]
fn propose_guideline(rule: String) -> Result<(), String> {
    let mut state = State::get();
    state.community_guidelines.push(CommunityGuideline { rule });
    state.set(state);
    Ok(())
}

#[query]
fn vote_guideline(rule_id: usize, vote: bool) -> Result<(), String> {
    let mut state = State::get();
    let voter = id().to_string();

    // Find the user who cast the vote
    let user = state.users.iter_mut().find(|u| u.id == voter);

    // Calculate the weight of the vote based on user reputation
    let weight = if let Some(user) = user {
        user.reputation as f32
    } else {
        1.0 // Default weight for unknown users
    };

    // Update the guideline's vote count and weighted vote sum
    let guideline = &mut state.community_guidelines[rule_id];
    guideline.votes += 1;
    guideline.weighted_votes += weight;

    state.set(state);
    Ok(())
}

#[query]
fn get_guidelines() -> Vec<CommunityGuideline> {
    State::get().community_guidelines.clone()
}

#[query]
fn update_reputation(user_id: String, delta: i64) -> Result<(), String> {
    let mut state = State::get();
    let user = state.users.iter_mut().find(|u| u.id == user_id);
    if let Some(user) = user {
        user.reputation += delta;
    }
    state.set(state);
    Ok(())
}

#[query]
fn moderate_content(content_id: u64) -> Result<(), String> {
    let state = State::get();
    let votes = state.votes.iter().filter(|v| v.0 == content_id).count();
    let content = state.contents.iter().find(|c| c.id == content_id);
    if let Some(content) = content {
        if votes > 5 { // Adjust threshold as needed
            for guideline in &state.community_guidelines {
                if content.content.contains(&guideline.rule) {
                    // Remove content
                    state.contents.retain(|c| c.id != content_id);
                    // Update user reputations
                    let reports = state.reports.iter().filter(|r| r.content_id == content_id);
                    for report in reports {
                        update_reputation(report.reporter.clone(), 10); // Adjust reward
                    }
                    break;
                }
            }
        }
    }
    Ok(())
}