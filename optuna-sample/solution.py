import os
import time

# Default values for x and y if the environment variables are not set
DEFAULT_X = 1
DEFAULT_Y = 1.0

# Get the parameters from the environment variables
x = int(os.getenv("AHC_X") or DEFAULT_X)
y = float(os.getenv("AHC_Y") or DEFAULT_Y)

# f(x, y) = x^2 + y^2
f = x * x + y * y

# Simulate the time taken to run the program
time.sleep(1)

# Print the score
score = int(f * 1000000)
print(f"Score = {score}")
