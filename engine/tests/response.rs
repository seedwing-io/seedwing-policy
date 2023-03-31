use seedwing_policy_engine::lang::Severity;
use seedwing_policy_engine::runtime::Response;
use serde_json::json;

#[test]
fn test_collapse() {
    let response: Response =
        serde_json::from_str(include_str!("response-data/proxy-jdom.json")).unwrap();

    // should be error
    assert_eq!(response.severity, Severity::Error);

    let response = response.collapse(Severity::Error);

    // should still be error after collapsing
    assert_eq!(response.severity, Severity::Error);
    assert_eq!(
        serde_json::to_value(&response).unwrap(),
        json!({
            "name":{
                "pattern":"test::not-affected"
            },
            "input":{
                "hash":"02bd61a725e8af9b0176b43bf29816d0c748b8ab951385bd127be37489325a0a",
                "purl":"pkg:maven/org.jdom/jdom@1.1.3?type=jar&repository_url=https%3A%2F%2Frepo.maven.apache.org%2Fmaven2",
                "url":"https://repo.maven.apache.org/maven2/org/jdom/jdom/1.1.3/jdom-1.1.3.jar"
            },
            "severity":"error",
            "reason":"Because not all fields were satisfied",
            "rationale":[
                {
                    "name":{
                        "pattern":"list::none"
                    },
                    "bindings":{
                        "pattern":{
                            "status":[
                                [
                                    "affected",
                                    "under_investigation"
                                ]
                            ]
                        },
                        "refinement":{
                            "statements":[
                                {
                                    "status":[
                                        [
                                            "affected",
                                            "under_investigation"
                                        ]
                                    ]
                                }
                            ]
                        },
                        "terms":[

                        ]
                    },
                    "input":[
                        {
                            "action_statement":"Review GHSA-2363-cqg2-863c for details on the appropriate action",
                            "action_statement_timestamp": "2023-03-31T13:38:32.159612889Z",
                            "products":[
                                "pkg:maven/org.jdom/jdom@1.1.2",
                                "pkg:maven/org.jdom/jdom@1.1",
                                "pkg:maven/org.jdom/jdom@1.1.3",
                                "pkg:maven/org.jdom/jdom@2.0.0",
                                "pkg:maven/org.jdom/jdom@2.0.1",
                                "pkg:maven/org.jdom/jdom@2.0.2"
                            ],
                            "status":"affected",
                            "status_notes":"Open Source Vulnerabilities (OSV) found vulnerabilities",
                            "timestamp": "2023-03-28T05:39:14.823719Z",
                            "vuln_description":"XML External Entity (XXE) Injection in JDOM",
                            "vulnerability":"GHSA-2363-cqg2-863c"
                        }
                    ],
                    "severity":"error",
                    "reason":"The input does not satisfy the function"
                }
            ]
        })
    );
}
