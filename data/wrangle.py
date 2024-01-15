import json
from collections import defaultdict
import string
import os
from yaml import load, dump

try:
    from yaml import CLoader as Loader, CDumper as Dumper
except ImportError:
    from yaml import Loader, Dumper


def split_by_continent():
    with open("country_continent.json") as f:
        data = json.load(f)
    res = defaultdict(list)
    for obj in data:
        res[obj["continent"]].append(obj["country"])
    for v in res.values():
        v.sort()
    with open("continent_to_country.json", "w") as f:
        json.dump(res, f)


def continent_fixup():
    with open("country_continent.json") as f:
        countries = json.load(f)
    for c in countries:
        if c["country"] == "Holy See (Vatican City State)":
            c["country"] = "Vatican City"
    countries.sort(key=lambda x: x["country"])
    with open("country_continent.json", "w") as f:
        json.dump(countries, f, indent=2)


def capital_fixup():
    with open("country_capital.json") as f:
        capitals = json.load(f)
    capitals = {c["country"]: c["city"] for c in capitals}

    capitals["Belgium"] = "Brussels"
    capitals["Colombia"] = "Bogota"
    capitals["Finland"] = "Helsinki"
    capitals.pop("Holy See (Vatican City State)", None)
    capitals["Vatican City"] = "Vatican City"
    capitals["Luxembourg"] = "Luxembourg"
    capitals["Marshall Islands"] = "Majuro"
    capitals["Mexico"] = "Mexico City"
    capitals["Myanmar"] = "Naypyidaw"
    capitals["Palau"] = "Ngerulmud"
    capitals["Palestine"] = "Ramallah"
    capitals["Panama"] = "Panama City"
    capitals["Sri Lanka"] = "Colombo"
    capitals["Togo"] = "Lome"
    capitals["Western Sahara"] = "Laayoune"
    capitals["Chile"] = "Santiago"
    capitals["Cuba"] = "Havana"
    capitals["Dominican Republic"] = "Santo Domingo"
    capitals["Guatemala"] = "Guatemala City"
    capitals["Cook Islands"] = "Avarua"

    capitals = [{"country": k, "city": v} for k, v in capitals.items()]
    capitals.sort(key=lambda x: x["country"])
    with open("country_capital.json", "w") as f:
        json.dump(capitals, f, indent=2)


def continent_to_capitals():
    with open("continent_to_country.json") as f:
        continents = json.load(f)
    with open("country_capital.json") as f:
        capitals = json.load(f)
    printable = set(string.printable)
    fprint = lambda s: "".join(ss for ss in s if ss in printable)
    for cont, countries in continents.items():
        cs = [cc for cc in capitals if cc["country"] in countries and cc["city"]]
        cs = [{fprint(k): fprint(v) for k, v in c.items()} for c in cs]
        if len(cs) == 0:
            continue
        cont = cont.replace(" ", "_").lower()
        with open(f"{cont}_capitals.json", "w") as f:
            json.dump(cs, f, indent=2)


def continent_capitals_questions():
    for p in os.listdir("."):
        if not p.endswith("_capitals.json"):
            continue
        with open(p) as f:
            data = json.load(f)
        continent = p.removesuffix("_capitals.json")
        items = [
            {
                "question": d["country"],
                "answers": [d["city"]],
            }
            for d in data
        ]
        items.sort(key=lambda x: x["question"])
        questions = {
            "name": "capitals",
            "type_": "default",
            "data": {
                "question_prefix": "What is the capital of ",
                "items": items,
            },
        }
        with open(f"questions/{continent}_capitals.yaml", "w") as f:
            dump(questions, f, Dumper=Dumper)


def gen_capital_questions():
    continent_fixup()
    split_by_continent()
    capital_fixup()    
    continent_to_capitals()
    continent_capitals_questions()

gen_capital_questions()
