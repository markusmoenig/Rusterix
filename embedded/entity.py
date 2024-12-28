import pickle

class Entity:
    def __init__(self, initial_attributes=None):
        """Initializes an entity with attributes."""
        self.attributes = initial_attributes or {}

    def update_attribute(self, key, value):
        """Updates or adds an attribute."""
        self.attributes[key] = value

    def get_all_attributes(self):
        """Returns all attributes."""
        return self.attributes

    def serialize(self):
        """Serializes the Entity to a pickle string."""
        return pickle.dumps(self)

    @staticmethod
    def deserialize(data):
        """Deserializes an Entity from a pickle string."""
        return pickle.loads(data)

    def debug(self):
        """Prints all attributes of the entity."""
        print("Entity attributes:")
        for key, value in self.attributes.items():
            print(f" - {key}: {value}")
