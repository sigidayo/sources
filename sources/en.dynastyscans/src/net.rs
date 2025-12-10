use aidoku::{
    AidokuError,
    alloc::{String, Vec},
    imports::{
        net::{Request, Response},
        std::sleep,
    },
    println,
};
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub struct BatchedRequest {
    requests: Vec<InnerRequest>,
    indexes: Vec<usize>,
}

#[derive(Debug)]
struct InnerRequest {
    url: String,
    response: Option<Response>,
}

impl BatchedRequest {
    pub fn new(urls: Vec<String>) -> Self {
        let requests: Vec<InnerRequest> = urls.into_iter().map(InnerRequest::new).collect();
        Self {
            indexes: (0..requests.len()).collect(),
            requests,
        }
    }

    pub fn get_jsons<T: DeserializeOwned>(mut self) -> Result<Vec<T>, AidokuError> {
        self.send_requests()?;
        self.inner_as_jsons()
    }

    fn send_requests(&mut self) -> Result<(), AidokuError> {
        let mut iterations = 0;
        let mut completed_flag = self.is_completed();

        while !completed_flag {
            if iterations > 3 {
                return Err(AidokuError::message("Too many request attempts"));
            }

            let requests = self.inner_as_requests();
            let responses = Request::send_all(requests)
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;

            for (response, idx) in responses.into_iter().zip(self.indexes.clone()) {
                if response.status_code() == 200 {
                    self.requests
                        .get_mut(idx)
                        .expect("idx should always fall within bounds")
                        .update_response(response);
                    let to_remove_idx = self
                        .indexes
                        .iter()
                        .position(|i| *i == idx)
                        .expect("Should not fail");
                    self.indexes.remove(to_remove_idx);
                } else {
                    println!(
                        "bad response, {idx}: {response:?}, code: {}",
                        response.status_code()
                    );
                }
            }

            if self.is_completed() {
                completed_flag = true
            } else {
                sleep(1);
                iterations += 1;
            }
        }
        println!("Successfully sent all requests");

        Ok(())
    }

    fn is_completed(&self) -> bool {
        self.requests
            .iter()
            .filter(|r| r.response.is_none())
            .collect::<Vec<_>>()
            .is_empty()
    }

    fn inner_as_jsons<T: DeserializeOwned>(self) -> Result<Vec<T>, AidokuError> {
        self.requests
            .into_iter()
            .map(|r| r.response.unwrap().get_json_owned())
            .collect::<Result<Vec<_>, _>>()
    }

    fn inner_as_requests(&self) -> Vec<Request> {
        self.requests
            .iter()
            .filter(|r| r.response.is_none())
            .map(|r| r.request())
            .collect()
    }
}

impl InnerRequest {
    fn new(url: String) -> Self {
        Self {
            url,
            response: None,
        }
    }

    fn request(&self) -> Request {
        Request::get(&self.url).expect("this should never fail")
    }

    fn update_response(&mut self, response: Response) {
        println!("Valid response for {}", self.url);
        self.response = Some(response);
    }
}
