# Examples

Each section links to its input and shows the rendered diagram.

## Sequence

Input: [sequence.txt](sequence.txt)

```text
+---------+             +---------+ 
| student |             | master  | 
+---------+             +---------+ 
     |                       |      
     | enter room            |      
     |-----------------------|      
     |                       |      
     | ask the way           |      
     |---------------------->|      
     |                       |      
     | show the way          |      
     |<......................|      
     |                       |      
     | student knows the way |      
     |.......................|      
     |                       |      
```

## Class

Input: [class.txt](class.txt)

```text
+---------+
| Person  |
+---------+
     A
     |
     |
+---------+     +---------------+
| Master  |*----| SecretScroll  |
+---------+     +---------------+
     ^
     |
     |
+---------+     +---------+
| Student |...|>| Trainee |
+---------+     +---------+
```

## Usecase

Input: [usecase.txt](usecase.txt)

```text
             + Karate Dojo --------------------+
    o        |                                 |        o
   /|\-----+-+-( Practice Basic Kata )         | +-----/|\
   / \       |   :                             | |     / \
White Belt   |   : include                     | | Yellow Belt
             |   v                             | |
             | ( Bow & Observe Etiquette )     | |
             |                                 | |
             |                                 | |
             |                                 | |
             | ( Perform Controlled Sparring )-+-+
             |     ^                           |
             |     : extend                    |
             |     :                           |
             | ( Correct Stance & Form )       |
             |                                 |
             +---------------------------------+
```

## Architecture

Input: [architecture.txt](architecture.txt)

```text
+ Sources ------------------------------------------+
|                                                   |
| +-----------+     +-----------+     +-----------+ |
| | Source A  |     | Source B  |     | Source C  | |
| +-----------+     +-----------+     +-----------+ |
|       |                 |                 |       |
+-------+-----------------+-----------------+-------+
        |                 |                 |
+ Storage ----------------+-----------------+-------+
|       v                 v                 v       |
|  +---------+       +---------+      +-----------+ |
|  | DB One  |       | DB Two  |      | DB Three  | |
|  +---------+       +---------+      +-----------+ |
|                         |                         |
+-------------------------+-------------------------+
        +-----------------+
        |
+ Output -------------------------------------------+
|       v                                           |
|   +-------+        +---------+      +-----------+ |
|   | Brief |------->| Analyst |----->| Delivery  | |
|   +-------+        +---------+      +-----------+ |
|                                                   |
+---------------------------------------------------+
```
