import requests


def test():
    # body = {
    #     "method": "POST",
    #     "url": "http://localhost:8080",
    #     "body": {
    #         "method": "GET",
    #         "url": "http://google.com",
    #     }
    # }
    body = {
        "method": "GET",
        "url": "https://google.com",
        "body": {
            "method": "GET",
            "url": "http://google.com",
        }
    }
    r = requests.post("http://localhost:8080", json=body)

    print(r.status_code)


test()
