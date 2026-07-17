import requests

handle = "ar7e9"

users = requests.get(
    "https://codeforces.com/api/user.ratedList?activeOnly=false"
).json()["result"]

for i, user in enumerate(users, 1):
    if user["handle"].lower() == handle.lower():
        print("Rank:", i)
        print("Rating:", user["rating"])
        break
