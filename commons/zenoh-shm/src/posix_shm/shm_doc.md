**SHM key-value storage**

**Integration schema**

liveliness
interface
   |
Leader----events---->Consumers
   |                     |
   |---->In-SHM Index----|
         |    |    |
        buf  buf  buf

**Index description**

_Hash_

hash: key hash (hash function with speed prior to collisions)
pre_hash: xor of hash bytes

_In-SHM data structure_
       ____________________
       | pre_hash0   : u8 |  <------ this is virtual and fixed size
       |....              |
       | pre_hash255 : u8 |___
       |------------------|  |
                             |
             ________________|
       ______|______
       |  SHM buf  |
       |-----------|     
       |hash0 : u32|  <-------- this can be made resizable (create new -> fill with data -> replace actual)
       |....       |
       |hashN : u32|____
       |-----------|   |
       |shm desc 0 |   |
       |....       |   |
   ____|shm desc N |<--|
   |   |-----------|
   |
   |   _______________
   |-->|   SHM buf   |
       |-------------|
       | key | value |
       |-------------|
    
**_Locking, synchronization and robustness_**

Fully lockfree operation
Synchronization issues give "previous tick" state to the consumer in the worst case
SHM mechanics guarantees robustness

**_Fragmentation_**

Fragmentation issues slightly reduce efficiency, but could be amortized by having proper lazy defragmenting algos

**_Algo complexity_**

Insertion
Average case: O(1)
Worst-case (extremely rare): O(n)

Removal: O(n)

Search: O(n)
(Will take worst-case of 397 iterations to pick an element in 100k pool)

**_Memory overhead in example_**

Assumptions
We have 100 nodes and totally 100k of elements
let's say average key length is 50 bytes
let's say average data length is 50 bytes

Key-value data will be 100k * 100 bytes = 9.5MB

Current case
Total (tree indexes not evaluated)
100 nodes * 9.5 Mb = 950Mb

SHM
100k elems / 256 pre_hashes = 397 elems per hashtable
397 elems * 10 bytes (elem size) = 3970 bytes = 4k (aligned to page size)
4k * 256 tables = 1MB overall worst-case index size

Total
9.5 Mb of shared key-value data + 1 Mb shared index = 10.5 Mb