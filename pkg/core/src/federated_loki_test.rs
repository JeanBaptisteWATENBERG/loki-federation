#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::future;
    use generic_loki_client::{Data, LabelResponse, LokiClient, LokiError, Response, ResultType, SerieResponse, VectorOrStream};
    use mockall::{automock, predicate};
    use async_trait::async_trait;

    use crate::datasources_provider::{MockDataSourcesProvider, MockDataSourceInstance, DataSource, HttpDataSource};
    use crate::federated_loki::*;


    #[derive(Debug, Clone)]
    struct TestLokiClient {}

    #[async_trait]
    #[automock]
    impl LokiClient for TestLokiClient {
        async fn query(&self, _: String, _: Option<i32>, _: Option<i64>, _: Option<generic_loki_client::Direction>) -> Result<Response, LokiError> { todo!() }
        async fn query_range(&self, _: String, _: i64, _: i64, _: Option<i32>, _: Option<generic_loki_client::Direction>, _: Option<String>, _: Option<String>) -> Result<Response, LokiError> { todo!() }
        async fn labels(&self, _: Option<i64>, _: Option<i64>) -> Result<LabelResponse, LokiError> { todo!() }
        async fn label_values(&self, _: String, _: Option<i64>, _: Option<i64>) -> Result<LabelResponse, LokiError> { todo!() }
        async fn series(&self, _: Option<Vec<String>>, _: Option<i64>, _: Option<i64>) -> Result<SerieResponse, LokiError> { todo!() }
    }

    fn sample_response(result: Vec<(String, String)>) -> Response {
        Response {
            status: "success".to_string(),
            data: Data {
                result_type: ResultType::Streams,
                result: vec![VectorOrStream {
                    stream: Some(HashMap::from([
                        ("label".to_string(), "value".to_string())
                    ])),
                    values: Some(result),
                    value: None,
                    metric: None,
                }],
            }
        }
    }

    fn mock_datasource_instance(client: MockTestLokiClient) -> MockDataSourceInstance {
        let ds_ctx = MockDataSourceInstance::new_context();

        ds_ctx.expect()
            .returning(|_| {
                MockDataSourceInstance::default()
            });

        let mut mock_ds = MockDataSourceInstance::new(DataSource::HttpDataSource(HttpDataSource {
            url: "http://localhost:3100".to_string()
        }));

        mock_ds.expect_get_client()
            .with()
            .return_once( || {
                Ok(Box::new(client))
            });
        mock_ds
    }

    fn mock_datasource_provider(client_a: MockTestLokiClient, client_b: MockTestLokiClient) -> MockDataSourcesProvider {
        let mocked_data_source_a = mock_datasource_instance(client_a);
        let mocked_data_source_b = mock_datasource_instance(client_b);

        let provider_ctx = MockDataSourcesProvider::new_context();

        provider_ctx.expect()
            .returning(|| {
                MockDataSourcesProvider::default()
            });

        let mut provider = MockDataSourcesProvider::new();

        provider.expect_get_data_sources()
            .return_once(move || Ok(vec![mocked_data_source_a, mocked_data_source_b]));
        provider
    }

    fn get_response_result(response: Response) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = vec![];
        for stream in response.data.result {
            for (key, value) in stream.values.unwrap() {
                result.push((key, value));
            }
        }
        result
    }

    #[tokio::test]
    async fn it_should_aggregate_query() {

        let mut mock_client_a: MockTestLokiClient = MockTestLokiClient::new();
        mock_client_a.expect_query()
            .with(predicate::eq("{job=\"foo\"}[5m]".to_string()), predicate::eq(None), predicate::eq(None), predicate::always())
            .return_once(|_, _, _, _| {
                Box::pin(future::ready(Ok(sample_response(vec![
                    ("4".to_string(), "d".to_string()),
                    ("2".to_string(), "b".to_string()),
                    ("1".to_string(), "a".to_string()),
                ]))))
            });

        let mut mock_client_b: MockTestLokiClient = MockTestLokiClient::new();
        mock_client_b.expect_query()
            .with(predicate::eq("{job=\"foo\"}[5m]".to_string()), predicate::eq(None), predicate::eq(None), predicate::always())
            .return_once(|_, _, _, _| {
                Box::pin(future::ready(Ok(sample_response(vec![
                    ("4".to_string(), "d".to_string()),
                    ("3".to_string(), "c".to_string()),
                    ("1".to_string(), "a".to_string()),
                ]))))
            });

        let loki = FederatedLoki::new(mock_datasource_provider(mock_client_a, mock_client_b));

        let aggregated_response = loki.query("{job=\"foo\"}[5m]".to_string(), None, None, None).await.unwrap();
        assert_eq!(get_response_result(aggregated_response), vec![
            ("4".to_string(), "d".to_string()),
            ("3".to_string(), "c".to_string()),
            ("2".to_string(), "b".to_string()),
            ("1".to_string(), "a".to_string()),
        ]);
    }
}