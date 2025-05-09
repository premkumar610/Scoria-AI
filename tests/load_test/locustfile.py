# load_test/locustfile.py

from locust import HttpUser, TaskSet, between, task, events
import gevent
import json
import numpy as np
from faker import Faker
from web3 import Web3
import solana.rpc.api
from solders.pubkey import Pubkey

fake = Faker()
sol_client = solana.rpc.api.Client("https://api.mainnet-beta.solana.com")
w3 = Web3(Web3.HTTPProvider("https://mainnet.infura.io/v3/KEY"))

class BlockchainTasks:
    @staticmethod
    def generate_solana_tx():
        return {
            "sender": str(Pubkey.from_seed(fake.binary(length=32))),
            "receiver": str(Pubkey.from_seed(fake.binary(length=32))),
            "lamports": np.random.randint(1, 1000000000),
            "model_hash": fake.sha256()
        }

    @staticmethod
    def generate_evm_tx():
        return {
            "from": w3.eth.account.create().address,
            "to": Web3.to_checksum_address(fake.ethereum_address()),
            "value": w3.to_wei(np.random.uniform(0.01, 100), 'ether'),
            "data": fake.binary(length=256)
        }

class AITasks:
    @staticmethod
    def generate_inference_payload():
        return {
            "model_version": "scoria-2.1.0",
            "input_data": [[np.random.rand() for _ in range(768)]],
            "precision": "fp16" if np.random.rand() > 0.5 else "int8"
        }

class ScoriaUser(HttpUser):
    host = "https://api.scoria.ai/v1"
    wait_time = between(0.5, 5)

    def on_start(self):
        self.client.verify = True
        self.client.headers.update({
            "Authorization": f"Bearer {os.getenv('SCORIA_API_KEY')}",
            "X-GPU-ID": "Tesla-V100-PCIE-32GB"
        })
        self.model_hash = "QmXg8jC..."

    @task(5)
    def submit_inference(self):
        payload = AITasks.generate_inference_payload()
        with self.client.post("/inference", 
                            json=payload,
                            catch_response=True) as response:
            if response.status_code != 202:
                response.failure(f"Status {response.status_code}")
            else:
                response.success()
                task_id = response.json()["task_id"]
                gevent.spawn(self.monitor_task, task_id)

    @task(3)
    def blockchain_verify(self):
        tx_data = BlockchainTasks.generate_solana_tx()
        with self.client.post("/blockchain/verify",
                            json=tx_data,
                            name="/blockchain/verify",
                            catch_response=True) as response:
            if "signature" not in response.json():
                response.failure("Missing blockchain signature")

    @task(2)
    def privacy_operation(self):
        with self.client.post("/privacy/aggregate",
                            json={"model_hashes": [self.model_hash]},
                            name="/privacy/aggregate",
                            catch_response=True) as response:
            if response.elapsed.total_seconds() > 30:
                response.failure("Aggregation timeout")

    def monitor_task(self, task_id):
        start_time = time.time()
        while time.time() - start_time < 120:
            with self.client.get(f"/tasks/{task_id}", 
                               name="/tasks/[id]",
                               catch_response=True) as response:
                if response.json().get("status") == "COMPLETED":
                    events.request.fire(
                        request_type="GET",
                        name="Task Completion",
                        response_time=(time.time() - start_time)*1000,
                        response_length=0
                    )
                    break
                elif response.json().get("status") == "FAILED":
                    response.failure("Task failed")
                    break
            gevent.sleep(5)

@events.init_command_line_parser.add_listener
def add_arguments(parser):
    parser.add_argument("--max-rps", type=int, default=1000, 
                      help="Max requests per second")
    parser.add_argument("--gpu-stress", action="store_true",
                      help="Enable CUDA kernel stress tests")

@events.test_start.add_listener
def setup_environment(environment, **kwargs):
    if environment.parsed_options.gpu_stress:
        import torch
        torch.cuda.init()
        print(f"GPU Stress Testing Enabled: {torch.cuda.get_device_name(0)}")
