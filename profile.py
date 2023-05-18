profile = open("profile", encoding = "utf8")
nb, np, tu = [], [], []
for line in profile:
    nbytes, npackets, timeused = [float(p) for p in line.split()]
    nb.append(nbytes)
    np.append(npackets)
    tu.append(timeused)

import numpy
import matplotlib.pyplot as plt

# plot number of packets vs. time
plt.plot(np, tu, 'o')
plt.show()
print("max time used", max(tu)/1000/1000)

# average time used per packet
tu = numpy.array(tu)
np = numpy.array(np)
print("Average time used per packet: %f" % (tu.sum() / np.sum()/1000/1000))
print("max time used per packet: %f" % (tu.max() / np[ numpy.where(tu == tu.max()) ]/1000/1000))