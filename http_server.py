from flask import jsonify, request, Flask
import random
import time

app = Flask(__name__)

# time to live in ms
TTL = 100


@app.route("/squareme")
def get():
    try:
        val = request.args.get('num')
        val = int(val, 10)
        delay = 50 + random.randint(50, 200)
        time.sleep(delay / 1000)
        if random.randint(0, 10) == 0:
            return jsonify({"error": "something went wrong"})
        else:
            return jsonify({"msg": val * val, "ttl(ms)": TTL + delay})
    except Exception as e:
        print(e)
        pass
