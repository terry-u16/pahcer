import os
import time

# Get the parameters from the environment variables
x = float(os.getenv("AHC_PARAM1"))
y = float(os.getenv("AHC_PARAM2"))

# f(x, y) = floor((x^2 + y^2) * 100000)
score = int((x * x + y * y) * 100000)

# Simulate the time taken to run the program
time.sleep(1)

# Print the score
print(f"Score = {score}")
