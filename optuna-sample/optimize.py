import json
import math  # noqa: F401
import os
import subprocess

import optuna


class Objective:
    def __init__(self) -> None:
        pass

    def __call__(self, trial: optuna.trial.Trial) -> float:
        # TODO: Write your parameters here
        # for more information, see https://optuna.readthedocs.io/en/stable/reference/generated/optuna.trial.Trial.html
        params = {
            "AHC_PARAM1": str(trial.suggest_float("param1", -5.0, 5.0, log=False)),
            "AHC_PARAM2": str(trial.suggest_float("param2", -5.0, 5.0, log=False)),
        }
        env = os.environ.copy()
        env.update(params)

        scores = []

        process = subprocess.Popen(
            ["pahcer", "run", "--json", "--shuffle", "--no-result-file"],
            stdout=subprocess.PIPE,
            env=env,
        )

        # see also: https://tech.preferred.jp/ja/blog/wilcoxonpruner/
        for line in process.stdout:
            result = json.loads(line)

            # If an error occurs, stop the process and raise an exception
            if result["error_message"] != "":
                process.send_signal(subprocess.signal.SIGINT)
                raise RuntimeError(result["error_message"])

            # for absolute score problems
            score = result["score"]

            # for relative score problems
            # score = math.log10(result["score"]) if result["score"] > 0.0 else 0.0

            seed = result["seed"]

            scores.append(score)
            trial.report(score, seed)

            if trial.should_prune():
                print(f"Trial {trial.number} pruned.")
                process.send_signal(subprocess.signal.SIGINT)

                # It is recommended to return the value of the objective function at the current step
                # instead of raising TrialPruned.
                # This is a workaround to report the evaluation information of the pruned Trial to Optuna.
                return sum(scores) / len(scores)

        return sum(scores) / len(scores)


# Set the direction to minimize
direction = "minimize"
# ...or maximize
# direction = "maximize"

study = optuna.create_study(
    direction=direction,
    study_name="optuna-sample",
    pruner=optuna.pruners.WilcoxonPruner(),
    sampler=optuna.samplers.TPESampler(),
)

objective = Objective()

# Set the number of trials to 100
study.optimize(objective, n_trials=100)

# ...or set the timeout to 60 seconds
# study.optimize(objective, timeout=60)

print(f"best params = {study.best_params}")
print(f"best score  = {study.best_value}")
