import psycopg2

def connect():
    """ Connect to the PostgreSQL database server """
    conn = None
    try:
      print('Connecting to the PostgreSQL database...')
      conn = psycopg2.connect(
        host="localhost",
        database="geotool",
        user="postgres",
        password="my-secret-pw")

      # create a cursor
      cur = conn.cursor()
        
	  # execute a statement
      print('PostgreSQL database version:')
      cur.execute('SELECT version()')

      # display the PostgreSQL database server version
      db_version = cur.fetchone()
      print(db_version)
       
	  # close the communication with the PostgreSQL
      cur.execute('SELECT * FROM users LIMIT 3')
      tables = cur.fetchall()
      for table in tables:
          print(table)

      cur.close()
    except (Exception, psycopg2.DatabaseError) as error:
        print(error)
    finally:
        if conn is not None:
            conn.close()
            print('Database connection closed.')


if __name__ == '__main__':
    connect()


