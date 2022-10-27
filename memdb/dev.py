import redis  # pip3 install redis


r = redis.Redis(
    host='localhost',
    port=6379, 
    password='')

r.set('foo', 'bar')
value = r.get('foo')
print(value)


