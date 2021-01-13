# Faulty Server Problem

A server is provided at http://faulty-server-htz-nbg1-1.wvservices.exchange:8080. It is supposed to return a JSON with a single 32-bit integer value. However, it is not very good at its job. It has following restrictions:

- It is somewhat slow
- Sometimes it returns an internal error with status 500
- Occasionally it times out with status 504
- It can handle only a certain number of concurrent connections, returning status 429 if the limit is exceeded

#### Possible faulty server responses

| Status | Response JSON                                 |
| ------ | --------------------------------------------- |
| 200    | `{ "value": 37 }`                             |
| 500    | `{ "error": "Internal server error" }`        |
| 504    | `{ "error": "Timed out" }`                    |
| 429    | `{ "error": "Too many concurrent requests" }` |

## Goal

The goal is to write a server application, providing a JSON API according to a specification below.

### Endpoints

#### Start a run

**Method:** `POST`

**Path:** `/runs`

**Request Body:**

A POST JSON body is expected with a `seconds` parameter. It describes the run lifetime.

```json
{
  "seconds": 30
}
```

**Description:**

Initiates the process of polling the faulty server called a 'run'.

**Response:**

For each run an ID must be generated and returned to a user right away, without waiting for the run finish.

```json
{
  "id": "<some generated ID>"
}
```

**Run goal:**

During the run the app should make as many requests to the faulty server as possible. A number of successful requests and a sum of received integer values should be calculated and saved.

**Faulty Server interaction:**

All requests to the faulty server must set header `X-Run-Id: <current run ID>`. If a request has failed, it should be retried. If a request has been made during the run, but the response came after its finish, its results must be ignored.

**Concurrency:**

Several concurrent runs must be allowed. Maximum number of concurrent runs should be parameterized by `MAX_CONCURRENT_RUNS` environment variable. Runs incoming after the maximum concurrency has been reached should await until any of the current runs is finished. Maximum number of runs pending execution should be parameterized by `MAX_PENDING_RUNS` environment variable. Further incoming runs should be discarded with HTTP status `429 Too Many Requests`.

#### Get run info by ID

**Method**: `GET`

**Path**: `/runs/{id}`

**Description**: Returns run info by the run `id`. The info should include: run status (`IN_PROGRESS` or `FINISHED`), a number of succesful responses received from the faulty server, and a sum of all received integer values.

**Response**:

```json
{
  "status": "IN_PROGRESS",
  "successful_responses_count": 17,
  "sum": 712
}
```
